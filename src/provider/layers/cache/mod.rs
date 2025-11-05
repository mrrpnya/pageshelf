/// A Layer that allows using Caches to temporarily store page info and Assets.
use std::sync::Arc;
pub mod owner;
pub mod page;
pub mod project;
use tracing::{debug, error, info};

use crate::{
    Asset, AssetError, AssetSource, Cache, CacheConnection,
    project::{Page, ProjectError, layer::ProjectSourceLayer, source::ProjectSource},
    provider::layers::cache::owner::CacheProjectOwner,
};

/// A Layer that caches page info and assets passed through it via Redis.
#[derive(Clone)]
pub struct CacheLayer<C: Cache> {
    cache: Arc<C>,
}

impl<C: Cache> CacheLayer<C> {
    pub fn from_cache(cache: C) -> Self {
        Self {
            cache: Arc::new(cache),
        }
    }
}

impl<PS: ProjectSource, C: Cache + 'static> ProjectSourceLayer<PS> for CacheLayer<C> {
    type Source = CacheLayerSource<PS, C>;

    fn wrap(&self, page_source: PS) -> Self::Source {
        Self::Source {
            upstream: page_source,
            cache: self.cache.clone(),
        }
    }
}

pub struct CacheLayerSource<PS: ProjectSource, C: Cache> {
    upstream: PS,
    cache: Arc<C>,
}

impl<PS: ProjectSource, C: Cache + 'static> ProjectSource for CacheLayerSource<PS, C> {
    type Owner<'a>
        = CacheProjectOwner<PS::Owner<'a>, C>
    where
        PS: 'a,
        C: 'a;
    /*async fn get_project(
        &self,
        owner: String,
        name: String,
        branch: String,
    ) -> Result<impl Page, ProjectError> {
        let mut conn = match self.cache.connect().await {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to create cache connection: {:?}", e);
                return Err(ProjectError::ProviderError);
            }
        };
        match self.upstream.get_project(owner, name, branch).await {
            Ok(page) => Ok({
                let version_key = format!(
                    "page:{}:{}:{}:version",
                    page.owner(),
                    page.name(),
                    page.branch()
                );
                match conn.get(&version_key).await {
                    Ok(v) => {
                        let version = std::str::from_utf8(&v);
                        if version.is_err() {
                            debug!("Page version is not UTF-8!");
                            return Err(ProjectError::ProviderError);
                        }
                        let version = version.unwrap();

                        if version != page.version() {
                            // Invalidate cache
                            info!(
                                "Page was updated (version: {}); Invalidating cache...",
                                version
                            );
                            let key = format!(
                                "page:{}:{}:{}:*",
                                page.owner(),
                                page.name(),
                                page.branch()
                            );
                            let _ = conn.delete(&key).await;

                            let _ = conn.set(&version_key, page.version().as_bytes()).await;
                        }
                    }
                    Err(e) => {
                        debug!("Unable to find page version in cache: {:?}", e);
                        let _ = conn.set(&version_key, page.version().as_bytes()).await;
                    }
                }
                CachePage {
                    upstream: page,
                    cache: self.cache.clone(),
                }
            }),
            Err(e) => Err(e),
        }
    }*/

    async fn all_owners<'a>(
        &'a self,
    ) -> Result<impl Iterator<Item = Self::Owner<'a>>, ProjectError> {
        self.upstream.all_owners().await.map(|i| {
            i.map(|o| CacheProjectOwner {
                upstream: o,
                cache: self.cache.clone(),
            })
        })
    }
}
