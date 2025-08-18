use std::{path::Path, str::FromStr};

use crate::{
    asset::{Asset, AssetError, AssetQueryable},
    conf::ServerConfig,
    page::{Page, PageError, PageSource, PageSourceFactory},
};
use forgejo_api::{Auth, Forgejo, structs::RepoSearchQuery};
use log::{error, warn};
use url::Url;

use super::assets::forgejo_direct::ForgejoDirectReadStorage;

enum Strategy {
    Direct,
}

pub struct ForgejoProvider {
    forgejo: Forgejo,
    strategy: Strategy,
    branches: Option<Vec<String>>,
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
    pub fn direct(forgejo: Forgejo, branches: Option<Vec<String>>) -> Self {
        Self {
            forgejo,
            strategy: Strategy::Direct,
            branches,
        }
    }
}

impl PageSource for ForgejoProvider {
    async fn page_at(
        &self,
        owner: &str,
        name: &str,
        channel: &str,
    ) -> Result<impl Page, PageError> {
        match &self.branches {
            Some(v) => {
                if !v.iter().any(|f| f == channel) {
                    warn!(
                        "Failed to access a Forgejo page: The branch {} is not in the list of accepted branches",
                        channel
                    );
                    warn!("Accepted branches are [{}]", v.join(", "));
                    return Err(PageError::NotFound);
                }
            }
            None => {}
        }

        match self.forgejo.repo_get(owner, name).await {
            Ok(v) => {
                // Verify that channel exists
                match self.forgejo.repo_get_branch(owner, name, channel).await {
                    Ok(_) => v,
                    Err(e) => {
                        error!(
                            "Failed to find Forgejo branch {} at {}/{} - {}",
                            channel, owner, name, e
                        );
                        return Err::<ForgejoPage, PageError>(PageError::ProviderError);
                    }
                };

                Ok(ForgejoPage {
                    storage: ForgejoDirectReadStorage::new(
                        &self.forgejo,
                        owner.to_string(),
                        name.to_string(),
                        channel.to_string(),
                    ),
                })
            }
            Err(e) => {
                error!(
                    "Failed to find Forgejo repository at {}/{} - {}",
                    owner, name, e
                );
                Err::<ForgejoPage, PageError>(PageError::ProviderError)
            }
        }
    }

    async fn pages(&self) -> Result<impl Iterator<Item = impl Page>, PageError> {
        let repo_search = match self
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
                limit: None,
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
                continue;
            }

            // TODO: Check on the login_name validity
            let user = repo.owner.unwrap().login_name.unwrap();
            let repo = repo.name.unwrap();

            match &self.branches {
                Some(v) => {
                    for branch in v {
                        let user = user.clone();
                        let repo = repo.clone();
                        let branch_result = self
                            .forgejo
                            .repo_get_branch(user.as_str(), repo.as_str(), branch.as_str())
                            .await;

                        match branch_result {
                            Ok(_) => {
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
                                continue;
                            }
                        }
                    }
                }
                None => {
                    // TODO: All Branches mode
                }
            }
        }

        Ok(pages.into_iter())
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Factory                                  */
/* -------------------------------------------------------------------------- */

#[derive(Clone)]
pub struct ForgejoProviderFactory {
    config: ServerConfig,
    url: Url,
}

impl ForgejoProviderFactory {
    // TODO: Set the error type in this result
    pub fn from_config(config: ServerConfig) -> Result<Self, ()> {
        let url = match url::Url::from_str(&config.upstream.url) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse Forgejo URL: {}", e);
                return Err(());
            }
        };

        Ok(Self { url, config })
    }
}
impl PageSourceFactory for ForgejoProviderFactory {
    type Source = ForgejoProvider;

    fn build(&self) -> Result<Self::Source, ()> {
        let fj = match Forgejo::new(Auth::None, self.url.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to create Forgejo authentication: {}", e);
                return Err(());
            }
        };

        Ok(ForgejoProvider::direct(
            fj,
            Some(self.config.upstream.branches.clone()),
        ))
    }
}

/*
impl PageSourceConfigurator for ForgejoProvider {
    type Source = ForgejoProvider;

    fn from_config(config: &crate::conf::ServerConfig) -> Self::Source {
        ForgejoProvider::direct(
            Forgejo::new(
                Auth::None,
                url::Url::from_str(&config.upstream.url).unwrap(),
            )
            .unwrap(),
            Some(config.upstream.branches.clone()),
        )
    }
}
*/
