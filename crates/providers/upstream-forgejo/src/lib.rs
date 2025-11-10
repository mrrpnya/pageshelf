//! Forgejo provider module; Allows integration with Forgejo instances.
//! For more information about Forgejo, see: https://forgejo.org/
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use color_eyre::{
    Section,
    eyre::{self, Context},
};

use std::{path::Path, sync::Arc};

use forgejo_api::{Auth, Forgejo, ForgejoError, structs::RepoGetRawFileQuery};
use pageshelf_core::upstream::{Upstream, UpstreamError};
use pageshelf_core::{event::EventBus, ext::Normalizable};
use scanner::ForgejoScanner;
use tracing::{error, info};

mod conf;

// TODO: Make private, but allow benchmark access
pub mod scanner;

pub use conf::*;

/// An [Upstream] that retrieves pages and assets from a Forgejo instance.
///
/// Performs periodic scans on the Forgejo instance so that it
pub struct ForgejoUpstream {
    forgejo: Arc<Forgejo>,
    analyzer: Arc<ForgejoScanner>,
}

impl ForgejoUpstream {
    /// Tries to create a [ForgejoUpstream] from a given configuration.
    ///
    /// It may fail as it has to connect with the Forgejo instance.
    pub fn create(
        config: &ForgejoUpstreamConfig,
        event_bus: EventBus,
    ) -> Result<Self, eyre::Report> {
        let forgejo = Arc::new(
            Forgejo::new(Auth::None, config.url.clone())
                .wrap_err("Failed to set up Forgejo provider")
                .suggestion("Check your Forgejo connection information?")?,
        );

        let analyzer = Arc::new(ForgejoScanner::start(
            forgejo.clone(),
            config.branches.clone(),
            config.scan_interval.unwrap_or(240),
            event_bus,
        ));

        Ok(Self { forgejo, analyzer })
    }
}

impl Upstream for ForgejoUpstream {
    async fn get_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
    ) -> Result<Arc<[u8]>, UpstreamError> {
        let path = path.normalized_relative();

        // Path validity check
        {
            let r = self.analyzer.data.repos.read().await;
            match r.get_channel(owner, project, channel) {
                Some(v) => {
                    if !v.valid_assets.contains(&path) {
                        return Err(UpstreamError::NotFound);
                    }
                    info!("Asset confirmed to be in the repo");
                }
                None => {
                    return Err(UpstreamError::NotFound);
                }
            }
        }

        let path = match path.to_str() {
            Some(v) => v,
            None => {
                return Err(UpstreamError::ProviderError);
            }
        };

        // Path confirmed :3

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

    async fn get_page_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<String, UpstreamError> {
        let repos = self.analyzer.data.repos.read().await;
        match repos.get_channel(owner, project, channel) {
            Some(c) => Ok(c.version.clone()),
            None => Err(UpstreamError::NotFound),
        }
    }

    async fn list_owners(&self) -> Result<Arc<[String]>, UpstreamError> {
        let repos = self.analyzer.data.repos.read().await;
        Ok(Arc::from(
            repos
                .owners
                .keys()
                .map(|f| f.to_string())
                .collect::<Vec<_>>(),
        ))
    }

    async fn list_projects(&self, owner: &str) -> Result<Arc<[String]>, UpstreamError> {
        let repos = self.analyzer.data.repos.read().await;
        Ok(Arc::from(
            repos
                .projects
                .keys()
                .filter(|f| &*f.owner == owner)
                .map(|f| f.project.to_string())
                .collect::<Vec<_>>(),
        ))
    }

    async fn list_channels(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Arc<[String]>, UpstreamError> {
        let repos = self.analyzer.data.repos.read().await;
        Ok(Arc::from(
            repos
                .channels
                .keys()
                .filter(|f| &*f.owner == owner && &*f.project == project)
                .map(|f| f.project.to_string())
                .collect::<Vec<_>>(),
        ))
    }
}
