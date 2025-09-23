mod asset_direct;
mod scanner;

use std::{path::Path, str::FromStr, sync::Arc};

use crate::{
    conf::ServerConfig,
    {Asset, AssetError, AssetSource}, {Page, PageError, PageSource, PageSourceFactory},
};
use forgejo_api::{Auth, Forgejo};
use log::{error, warn};
use scanner::ForgejoScanner;

use asset_direct::ForgejoDirectReadStorage;

pub struct ForgejoProvider {
    forgejo: Arc<Forgejo>,
    analyzer: Arc<ForgejoScanner>,
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

impl<'a> AssetSource for ForgejoPage<'a> {
    async fn get_asset(&self, path: &Path) -> Result<impl Asset, AssetError> {
        self.storage.get_asset(path).await
    }
}

impl ForgejoProvider {
    pub fn new(forgejo: Arc<Forgejo>, analyzer: Arc<ForgejoScanner>) -> Self {
        Self { forgejo, analyzer }
    }
}

impl PageSource for ForgejoProvider {
    async fn page_at(
        &self,
        owner: String,
        name: String,
        channel: String,
    ) -> Result<impl Page, PageError> {
        if !self
            .analyzer
            .data
            .target_branches
            .iter()
            .any(|f| f == &channel)
        {
            warn!(
                "Failed to access a Forgejo page: The branch {} is not in the list of accepted branches",
                channel
            );
            warn!(
                "Accepted branches are [{}]",
                self.analyzer.data.target_branches.join(", ")
            );
            return Err(PageError::NotFound);
        }

        let repos = self.analyzer.data.repos.read().await;

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
        let repos = self.analyzer.data.repos.read().await;

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
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Factory                                  */
/* -------------------------------------------------------------------------- */

#[derive(Clone)]
pub struct ForgejoProviderFactory {
    analyzer: Arc<ForgejoScanner>,
    forgejo: Arc<Forgejo>,
}

impl ForgejoProviderFactory {
    pub fn from_config(config: ServerConfig) -> Option<Self> {
        let url = match url::Url::from_str(&config.upstream.url) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse Forgejo URL: {}", e);
                return None;
            }
        };

        let fj = Arc::new(match Forgejo::new(Auth::None, url.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to create Forgejo authentication: {}", e);
                return None;
            }
        });

        let mut branches = config.upstream.branches.clone();
        if branches.is_empty() {
            branches.push("pages".to_string());
        }

        Some(Self {
            forgejo: fj.clone(),
            analyzer: Arc::new(ForgejoScanner::start(
                fj,
                branches,
                config.upstream.poll_interval.unwrap_or(240),
            )),
        })
    }
}

impl PageSourceFactory for ForgejoProviderFactory {
    type Source = ForgejoProvider;

    fn build(&self) -> Self::Source {
        ForgejoProvider::new(self.forgejo.clone(), self.analyzer.clone())
    }
}
