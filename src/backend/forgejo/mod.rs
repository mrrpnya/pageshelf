mod asset_direct;
mod scan;

use std::{
    path::Path,
    str::FromStr,
    sync::Arc,
};

use crate::{
    asset::{Asset, AssetError, AssetQueryable},
    conf::ServerConfig,
    page::{Page, PageError, PageSource, PageSourceFactory},
};
use forgejo_api::{Auth, Forgejo};
use log::{error, warn};
use scan::ForgejoAnalyzer;

use asset_direct::ForgejoDirectReadStorage;

enum Strategy {
    Direct,
}

pub struct ForgejoProvider {
    forgejo: Arc<Forgejo>,
    strategy: Strategy,
    analyzer: Arc<ForgejoAnalyzer>,
}

struct ForgejoPage<'a> {
    storage: ForgejoDirectReadStorage<'a>,
}

impl<'a> Page for ForgejoPage<'a> {
    fn name(&self) -> &str {
        self.storage.repo()
    }

    fn branch(&self) -> &str {
        self.storage.branch()
    }

    fn owner(&self) -> &str {
        self.storage.owner()
    }

    fn version(&self) -> &str {
        self.storage.version()
    }
}

impl<'a> AssetQueryable for ForgejoPage<'a> {
    async fn asset_at(&self, path: &Path) -> Result<impl Asset, AssetError> {
        self.storage.asset_at(path).await
    }

    fn assets(&self) -> Result<impl Iterator<Item = impl Asset>, AssetError> {
        self.storage.assets()
    }
}

impl ForgejoProvider {
    pub fn direct(forgejo: Arc<Forgejo>, analyzer: Arc<ForgejoAnalyzer>) -> Self {
        let s = Self {
            forgejo: forgejo,
            strategy: Strategy::Direct,
            analyzer,
        };

        s
    }
}

impl PageSource for ForgejoProvider {
    async fn page_at(
        &self,
        owner: String,
        name: String,
        channel: String,
    ) -> Result<impl Page, PageError> {
        if !self.analyzer.target_branches.iter().any(|f| f == &channel) {
            warn!(
                "Failed to access a Forgejo page: The branch {} is not in the list of accepted branches",
                channel
            );
            warn!(
                "Accepted branches are [{}]",
                self.analyzer.target_branches.join(", ")
            );
            return Err(PageError::NotFound);
        }

        let repos = self.analyzer.repos.read().await;

        match repos.get(&(owner.clone(), name.clone(), channel.clone())) {
            Some(v) => Ok(ForgejoPage {
                storage: ForgejoDirectReadStorage::new(
                    &self.forgejo,
                    owner.to_string(),
                    name.to_string(),
                    channel.to_string(),
                    v.version.clone(),
                ),
            }),
            None => {
                error!(
                    "Failed to find Forgejo repository at {}/{}:{}",
                    owner, name, channel
                );
                Err::<ForgejoPage, PageError>(PageError::ProviderError)
            }
        }
    }

    async fn pages(&self) -> Result<impl Iterator<Item = impl Page>, PageError> {
        let repos = self.analyzer.repos.read().await;

        let mut pages: Vec<ForgejoPage> = vec![];

        for repo in repos.keys() {
            pages.push(ForgejoPage {
                storage: ForgejoDirectReadStorage::new(
                    &self.forgejo,
                    repo.0.to_string(),
                    repo.1.to_string(),
                    repo.2.to_string(),
                    repos[repo].version.clone(),
                ),
            });
        }

        Ok(pages.into_iter())

        /*let repo_search = match self
            .forgejo
            .repo_search(RepoSearchQuery {
                q: None,
                topic: None,
                include_desc: None,
                uid: None,
                priority_owner_id: None,
                team_id: None,
                starred_by: None,
                private: None,
                is_private: None,
                template: None,
                archived: Some(false),
                mode: None,
                exclusive: None,
                sort: None,
                order: None,
                page: None,
                limit: Some(999999),
            })
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to search for Forgejo repositories: {}", e);
                return Err(PageError::ProviderError);
            }
        };

        let repos = match repo_search.data {
            Some(v) => v,
            None => {
                error!("Failed to search for Forgejo repositories (No data)");
                return Err(PageError::ProviderError);
            }
        };

        let mut pages: Vec<ForgejoPage> = Vec::new();

        for repo in repos {
            if repo.name.is_none() || repo.owner.is_none() {
                warn!(
                    "Repo {:?}/{:?} is invalid, skipping check...",
                    repo.owner, repo.name
                );
                continue;
            }

            let user = repo.owner.unwrap().login.unwrap();
            let repo = repo.name.unwrap();

            info!("Scanning repo {}/{}...", user.as_str(), repo.as_str());

            for branch in self.analyzer.target_branches.clone() {
                let user = user.clone();
                let repo = repo.clone();
                let branch_result = self
                    .forgejo
                    .repo_get_branch(user.as_str(), repo.as_str(), branch.as_str())
                    .await;
                debug!(
                    "Getting repo branch {}/{}:{}",
                    user.as_str(),
                    repo.as_str(),
                    branch.as_str()
                );

                match branch_result {
                    Ok(_) => {
                        info!(
                            "Found page at: {}/{}:{}",
                            user.as_str(),
                            repo.as_str(),
                            branch.as_str()
                        );
                        pages.push(ForgejoPage {
                            storage: ForgejoDirectReadStorage::new(
                                &self.forgejo,
                                user,
                                repo,
                                branch.to_string(),
                            ),
                        });
                    }
                    Err(e) => {
                        error!("Failed to get branch: {}", e);
                        continue;
                    }
                }
            }
        }

        Ok(pages.into_iter())*/
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Factory                                  */
/* -------------------------------------------------------------------------- */

#[derive(Clone)]
pub struct ForgejoProviderFactory {
    analyzer: Arc<ForgejoAnalyzer>,
    forgejo: Arc<Forgejo>,
}

impl ForgejoProviderFactory {
    pub fn from_config(config: ServerConfig) -> Result<Self, ()> {
        let url = match url::Url::from_str(&config.upstream.url) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse Forgejo URL: {}", e);
                return Err(());
            }
        };

        let fj = Arc::new(match Forgejo::new(Auth::None, url.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to create Forgejo authentication: {}", e);
                return Err(());
            }
        });

        let mut branches = config.upstream.branches.clone();
        if branches.len() == 0 {
            branches.push("pages".to_string());
        }

        Ok(Self {
            forgejo: fj.clone(),
            analyzer: Arc::new(ForgejoAnalyzer::start(fj, branches, config.upstream.poll_interval.unwrap_or(240)))
        })
    }
}

impl PageSourceFactory for ForgejoProviderFactory {
    type Source = ForgejoProvider;

    fn build(&self) -> Result<Self::Source, ()> {
        Ok(ForgejoProvider::direct(
            self.forgejo.clone(),
            self.analyzer.clone(),
        ))
    }
}
