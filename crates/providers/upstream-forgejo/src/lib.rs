//! Forgejo provider module; Allows integration with Forgejo instances.
//! For more information about Forgejo, see: https://forgejo.org/
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use color_eyre::{
    Section,
    eyre::{self, Context},
};
use futures::{
    StreamExt,
    stream::{self},
};

use std::{path::Path, sync::Arc};

use forgejo_api::{
    Auth, Forgejo, ForgejoError,
    structs::{GetTreeQuery, RepoGetRawFileQuery, RepoListBranchesQuery, RepoSearchQuery},
};
use pageshelf_core::ext::Normalizable;
use pageshelf_core::upstream::{
    UpstreamError,
    source::{
        AssetListSource, AssetSource, PageListSource, PageVersionSource, VersionedAssetSource,
    },
};
use tracing::{debug, error, info};

mod conf;

pub use conf::*;

/// Retrieves data (naively) from a Forgejo instance.
///
/// Recommended to wrap it in a [PolledSource](pageshelf_core::upstream::managers::PolledSource) for practical use.
pub struct ForgejoRawSource {
    forgejo: Arc<Forgejo>,
    valid_channels: Vec<String>,
}

impl ForgejoRawSource {
    /// Tries to create a [ForgejoUpstream] from a given configuration.
    ///
    /// It may fail as it has to connect with the Forgejo instance.
    pub fn create(
        config: &ForgejoUpstreamConfig,
        valid_channels: Vec<String>,
    ) -> Result<Self, eyre::Report> {
        let forgejo = Arc::new(
            Forgejo::new(Auth::None, config.url.clone())
                .wrap_err("Failed to set up Forgejo provider")
                .suggestion("Check your Forgejo connection information?")?,
        );

        Ok(Self {
            forgejo,
            valid_channels,
        })
    }
}
impl PageListSource for ForgejoRawSource {
    async fn list_pages(
        &self,
    ) -> Result<Arc<(Arc<[String]>, Arc<[String]>, Arc<[String]>)>, UpstreamError> {
        let search_results = self
            .forgejo
            .repo_search(RepoSearchQuery {
                q: None,
                topic: None,
                include_desc: None,
                uid: None,
                priority_owner_id: None,
                team_id: None,
                starred_by: None,
                private: Some(false),
                is_private: None,
                template: None,
                archived: None,
                mode: None,
                exclusive: None,
                sort: None,
                order: None,
                page: None,
                limit: Some(999_999),
            })
            .await
            .map_err(|e| {
                error!("Failed to search public repositories: {:?}", e);
                UpstreamError::ProviderError
            })?;

        let repos = match search_results.data {
            Some(r) => r,
            None => {
                error!("No repositories found in search results");
                return Ok(Arc::new((Arc::new([]), Arc::new([]), Arc::new([]))));
            }
        };

        // Prepare a stream of futures for fetching branches concurrently
        let results = stream::iter(repos)
            .map(|repo| {
                let valid_channels = self.valid_channels.clone();
                let forgejo = &self.forgejo;

                async move {
                    let owner = match repo.owner.as_ref().and_then(|o| o.login.as_ref()) {
                        Some(o) => o.clone(),
                        None => return Vec::new(),
                    };
                    let project = match repo.name.as_ref() {
                        Some(p) => p.clone(),
                        None => return Vec::new(),
                    };

                    match forgejo
                        .repo_list_branches(
                            &owner,
                            &project,
                            RepoListBranchesQuery {
                                page: None,
                                limit: None,
                            },
                        )
                        .await
                    {
                        Ok((_, branches)) => branches
                            .into_iter()
                            .filter_map(|b| {
                                if let Some(name) = b.name
                                    && valid_channels.contains(&name)
                                {
                                    Some((owner.clone(), project.clone(), name))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>(),
                        Err(_) => {
                            debug!("Failed to list branches for {}/{}", owner, project);
                            Vec::new()
                        }
                    }
                }
            })
            .buffer_unordered(50) // adjust concurrency limit as needed
            .collect::<Vec<Vec<(String, String, String)>>>()
            .await;

        // Flatten the results
        let flat: Vec<(String, String, String)> = results.into_iter().flatten().collect();

        let (owners_vec, projects_vec, channels_vec): (Vec<_>, Vec<_>, Vec<_>) = flat.unzip3();

        Ok(Arc::new((
            Arc::from(owners_vec.into_boxed_slice()),
            Arc::from(projects_vec.into_boxed_slice()),
            Arc::from(channels_vec.into_boxed_slice()),
        )))
    }
}

// Helper to unzip a vector of triples into three separate vectors
trait Unzip3<A, B, C> {
    fn unzip3(self) -> (Vec<A>, Vec<B>, Vec<C>);
}

impl<A, B, C> Unzip3<A, B, C> for Vec<(A, B, C)> {
    fn unzip3(self) -> (Vec<A>, Vec<B>, Vec<C>) {
        let mut a = Vec::with_capacity(self.len());
        let mut b = Vec::with_capacity(self.len());
        let mut c = Vec::with_capacity(self.len());
        for (x, y, z) in self {
            a.push(x);
            b.push(y);
            c.push(z);
        }
        (a, b, c)
    }
}

impl PageVersionSource for ForgejoRawSource {
    async fn get_page_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<String, UpstreamError> {
        let branch = self
            .forgejo
            .repo_get_branch(owner, project, channel)
            .await
            .map_err(|e| {
                error!(
                    "Failed to get branch for page version {}->{}:{} - {:?}",
                    owner, project, channel, e
                );
                UpstreamError::NotFound
            })?;

        if let Some(commit) = branch.commit
            && let Some(id) = commit.id
        {
            return Ok(id);
        }

        error!("Failed to get commit ID");

        Err(UpstreamError::NotFound)
    }
}

impl AssetSource for ForgejoRawSource {
    async fn get_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
    ) -> Result<Arc<[u8]>, UpstreamError> {
        if !self.valid_channels.contains(&channel.to_owned()) {
            info!("Channel {channel} not found");
            return Err(UpstreamError::NotFound);
        }

        let path = path.normalized_relative();

        let path = match path.to_str() {
            Some(v) => v,
            None => {
                return Err(UpstreamError::ProviderError);
            }
        };

        info!("Fetching raw data from Forgejo at {:?}", path);

        let result: Result<Vec<u8>, ForgejoError>;
        {
            result = self
                .forgejo
                .repo_get_raw_file(
                    owner,
                    project,
                    path,
                    RepoGetRawFileQuery {
                        r#ref: Some(channel.to_string()),
                    },
                )
                .await;
        }

        match result {
            Ok(v) => Ok(Arc::from(v)),
            Err(e) => {
                error!(
                    "Failed to find (raw) data file {} in Forgejo repository {}/{}:{} - {}",
                    path, owner, project, channel, e
                );
                Err(UpstreamError::NotFound)
            }
        }
    }
}

impl VersionedAssetSource for ForgejoRawSource {
    async fn get_version_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        version: &str,
        path: &Path,
    ) -> Result<Arc<[u8]>, UpstreamError> {
        if !self.valid_channels.contains(&channel.to_owned()) {
            info!("Channel {channel} not found");
            return Err(UpstreamError::NotFound);
        }

        let path = path.normalized_relative();

        let path = match path.to_str() {
            Some(v) => v,
            None => {
                return Err(UpstreamError::ProviderError);
            }
        };

        info!("Fetching raw data from Forgejo at {:?}", path);

        let result: Result<Vec<u8>, ForgejoError>;
        {
            result = self
                .forgejo
                .repo_get_raw_file(
                    owner,
                    project,
                    path,
                    RepoGetRawFileQuery {
                        r#ref: Some(version.to_string()),
                    },
                )
                .await;
        }

        match result {
            Ok(v) => Ok(Arc::from(v)),
            Err(e) => {
                error!(
                    "Failed to find (raw) data file {} in Forgejo repository {}/{}:{} - {}",
                    path, owner, project, channel, e
                );
                Err(UpstreamError::NotFound)
            }
        }
    }
}

impl AssetListSource for ForgejoRawSource {
    async fn list_assets(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<Arc<[String]>, UpstreamError> {
        if !self.valid_channels.contains(&channel.to_owned()) {
            info!("Invalid channel");
            return Err(UpstreamError::NotFound);
        }

        // Get the branch first to obtain the commit ID
        let branch = self
            .forgejo
            .repo_get_branch(owner, project, channel)
            .await
            .map_err(|e| {
                error!(
                    "Failed to get branch {}->{}:{} - {:?}",
                    owner, project, channel, e
                );
                UpstreamError::NotFound
            })?;

        let commit_id = branch.commit.and_then(|c| c.id).ok_or_else(|| {
            error!("Branch {}/{}:{} has no commit ID", owner, project, channel);
            UpstreamError::NotFound
        })?;

        // Get the tree recursively
        let tree = self
            .forgejo
            .get_tree(
                owner,
                project,
                &commit_id,
                GetTreeQuery {
                    recursive: Some(true),
                    page: None,
                    per_page: None,
                },
            )
            .await
            .map_err(|e| {
                error!(
                    "Failed to get tree for {}/{}:{} at commit {} - {:?}",
                    owner, project, channel, commit_id, e
                );
                UpstreamError::ProviderError
            })?;

        // Collect all blob (file) paths
        let mut files = Vec::new();
        if let Some(entries) = tree.tree {
            for entry in entries {
                if entry.r#type == Some("blob".to_string())
                    && let Some(path) = entry.path
                {
                    files.push(path);
                }
            }
        }

        Ok(Arc::from(files))
    }
}
