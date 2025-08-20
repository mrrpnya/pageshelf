// TODO: Implement Redis layer

use std::sync::Arc;

use log::{debug, error, info};
use redis::{AsyncCommands, Client, RedisError};

use crate::{
    asset::{Asset, AssetError, AssetQueryable},
    conf::ServerConfig,
    page::{Page, PageSource, PageSourceLayer},
};

#[derive(Clone)]
pub struct RedisLayer {
    client: Arc<redis::Client>,
}

impl RedisLayer {
    pub fn from_config(config: &ServerConfig) -> Result<Self, RedisError> {
        let address = format!("redis://{}:{}", config.redis.address, config.redis.port);
        match redis::Client::open(address) {
            Ok(v) => Ok(Self {
                client: Arc::new(v),
            }),
            Err(e) => {
                error!("Failed to set up Redis integration: {}", e);
                Err(e)
            }
        }
    }
}

impl<PS: PageSource> PageSourceLayer<PS> for RedisLayer {
    type Source = RedisCacheSource<PS>;

    fn wrap(&self, page_source: PS) -> Self::Source {
        Self::Source {
            upstream: page_source,
            client: self.client.clone(),
        }
    }
}

pub struct RedisCachePage<P: Page> {
    upstream: P,
    client: Arc<Client>,
}

impl<P: Page> Page for RedisCachePage<P> {
    fn name(&self) -> &str {
        self.upstream.name()
    }

    fn branch(&self) -> &str {
        self.upstream.branch()
    }

    fn owner(&self) -> &str {
        self.upstream.owner()
    }
}

pub enum RedisCacheAsset<A: Asset> {
    Hold(String),
    Load(A),
}

impl<A: Asset> Asset for RedisCacheAsset<A> {
    fn body(&self) -> String {
        match self {
            Self::Hold(data) => data.clone(),
            Self::Load(asset) => asset.body(),
        }
    }
}

impl<P: Page> AssetQueryable for RedisCachePage<P> {
    async fn asset_at(
        &self,
        path: &std::path::Path,
    ) -> Result<impl crate::asset::Asset, crate::asset::AssetError> {
        debug!("Connecting to Redis...");
        let mut conn = match self.client.get_multiplexed_async_connection().await {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to create multiplexed async Redis connection: {}", e);
                return Err(AssetError::ProviderError);
            }
        };
        let key = format!(
            "o{},r{},b{},a{}",
            self.name(),
            self.name(),
            self.branch(),
            path.to_str().unwrap()
        );
        debug!("Checking if asset \"{}\" asset is in cache...", key);
        match conn.get::<String, String>(key.clone()).await {
            Ok(v) => {
                info!("Cache hit: {:?}", path);
                Ok(RedisCacheAsset::Hold(v))
            }
            Err(e) => {
                info!("Cache miss (loading from upstream): {}", e);
                match self.upstream.asset_at(&path).await {
                    Ok(v) => {
                        conn.set::<String, String, String>(key, v.body()).await;
                        Ok(RedisCacheAsset::Load(v))
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    fn assets(
        &self,
    ) -> Result<impl Iterator<Item = impl crate::asset::Asset>, crate::asset::AssetError> {
        self.upstream.assets()
    }
}

pub struct RedisCacheSource<PS: PageSource> {
    upstream: PS,
    client: Arc<Client>,
}

impl<PS: PageSource> PageSource for RedisCacheSource<PS> {
    async fn page_at(
        &self,
        owner: &str,
        name: &str,
        branch: &str,
    ) -> Result<impl crate::page::Page, crate::page::PageError> {
        debug!("Wrapping page in a Redis abstraction...");
        match self.upstream.page_at(owner, name, branch).await {
            Ok(v) => Ok(RedisCachePage {
                upstream: v,
                client: self.client.clone(),
            }),
            Err(e) => Err(e),
        }
    }

    async fn pages(
        &self,
    ) -> Result<impl Iterator<Item = impl crate::page::Page>, crate::page::PageError> {
        self.upstream.pages().await
    }
}
