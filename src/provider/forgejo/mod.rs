//! Forgejo provider module; Allows integration with Forgejo instances.
//! For more information about Forgejo, see: https://forgejo.org/

mod owner;
mod page;
mod project;
mod scanner;

use std::{collections::HashMap, path::Path, str::FromStr, sync::Arc};

use crate::{
    Asset, AssetError, AssetSource,
    conf::ServerConfig,
    project::{
        Page, ProjectError, ProjectOwner, layer::ProjectSourceBuilder, source::ProjectSource,
    },
    provider::forgejo::{owner::ForgejoProjectOwner, project::ForgejoProject},
};
use forgejo_api::{Auth, Forgejo};
use scanner::ForgejoScanner;
use tracing::{error, warn};

pub struct ForgejoProvider {
    forgejo: Arc<Forgejo>,
    analyzer: Arc<ForgejoScanner>,
}

impl ForgejoProvider {
    pub fn new(forgejo: Arc<Forgejo>, analyzer: Arc<ForgejoScanner>) -> Self {
        Self { forgejo, analyzer }
    }
}

impl ProjectSource for ForgejoProvider {
    type Owner<'a> = ForgejoProjectOwner;

    async fn all_owners<'a>(
        &'a self,
    ) -> Result<impl Iterator<Item = Self::Owner<'a>>, ProjectError> {
        let mut owners: HashMap<String, ()> = HashMap::new();

        {
            let repos = self.analyzer.data.repos.read().await;

            for k in repos.owners.keys() {
                owners.insert(k.to_string(), ());
            }
        }

        let owners = owners.into_keys();

        Ok(owners.map(|f| {
            ForgejoProjectOwner::new(f.to_string(), self.forgejo.clone(), self.analyzer.clone())
        }))
    }

    async fn get_owner<'a>(&'a self, name: &str) -> Result<Option<Self::Owner<'a>>, ProjectError> {
        let owner: bool;
        {
            let repos = self.analyzer.data.repos.read().await;
            owner = repos.contains_owner(name);
        }

        match owner {
            true => Ok(Some(ForgejoProjectOwner::new(
                name.to_string(),
                self.forgejo.clone(),
                self.analyzer.clone(),
            ))),
            false => Ok(None),
        }
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
    pub fn from_config(config: &ServerConfig) -> Option<Self> {
        let url = match url::Url::from_str(&config.upstream.url) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse Forgejo URL: {}", e);
                return None;
            }
        };
        let forgejo = Arc::new(match Forgejo::new(Auth::None, url.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to create Forgejo authentication: {}", e);
                return None;
            }
        });

        Self::from_config_and_api(config, forgejo)
    }

    pub fn from_config_and_api(config: &ServerConfig, forgejo: Arc<Forgejo>) -> Option<Self> {
        let mut branches = config.upstream.branches.clone();
        if branches.is_empty() {
            branches.push("pages".to_string());
        }

        Some(Self {
            forgejo: forgejo.clone(),
            analyzer: Arc::new(ForgejoScanner::start(
                forgejo,
                branches,
                config.upstream.poll_interval.unwrap_or(240),
            )),
        })
    }
}

impl ProjectSourceBuilder for ForgejoProviderFactory {
    type Source = ForgejoProvider;

    fn build(&self) -> Self::Source {
        ForgejoProvider::new(self.forgejo.clone(), self.analyzer.clone())
    }
}
