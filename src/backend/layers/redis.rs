/// A Layer that allows using Redis to cache page info and Assets.
///
/// Redis is a high-speed in-memory cache with data durability.
/// It is useful for reducing queries upstream (especially when deployed off-site).
/// See: https://redis.io/ for more information about Redis itself.
use std::sync::Arc;

use log::{debug, error, info};
use redis::{AsyncCommands, Client, RedisError};

use crate::{
    asset::{Asset, AssetError, AssetQueryable},
    conf::ServerConfig,
    page::{Page, PageError, PageSource, PageSourceLayer},
};

/// A Layer that caches page info and assets passed through it via Redis.
#[derive(Clone)]
pub struct RedisLayer {
    client: Arc<redis::Client>,
    ttl: Option<i64>,
}

impl RedisLayer {
    pub fn from_config(config: &ServerConfig) -> Result<Self, RedisError> {
        let address = format!("redis://{}:{}", config.redis.address, config.redis.port);
        match redis::Client::open(address) {
            Ok(v) => Ok(Self {
                client: Arc::new(v),
                ttl: config.redis.ttl,
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
            ttl: self.ttl,
        }
    }
}

pub struct RedisCachePage<P: Page> {
    upstream: P,
    client: Arc<Client>,
    ttl: Option<i64>,
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

    fn version(&self) -> &str {
        self.upstream.version()
    }
}

pub enum RedisCacheAsset<A: Asset> {
    Hold(String),
    Load(A),
}

impl<A: Asset> Asset for RedisCacheAsset<A> {
    fn body(&self) -> &str {
        match self {
            Self::Hold(data) => &data,
            Self::Load(asset) => asset.body(),
        }
    }
}

pub enum RedisCacheAssetEither<A: Asset, B: Asset> {
    A(A),
    B(B),
}

impl<A: Asset, B: Asset> Asset for RedisCacheAssetEither<A, B> {
    fn body(&self) -> &str {
        match self {
            Self::A(data) => data.body(),
            Self::B(data) => data.body(),
        }
    }
}

pub enum RedisCacheAssetIterEither<
    A: Asset,
    B: Asset,
    AI: Iterator<Item = A>,
    BI: Iterator<Item = B>,
> {
    A(AI),
    B(BI),
}

impl<A: Asset, B: Asset, AI: Iterator<Item = A>, BI: Iterator<Item = B>> Iterator
    for RedisCacheAssetIterEither<A, B, AI, BI>
{
    type Item = RedisCacheAssetEither<A, B>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::A(data) => {
                if let Some(d) = data.next() {
                    return Some(RedisCacheAssetEither::<A, B>::A(d));
                }
                None
            }
            Self::B(data) => {
                if let Some(d) = data.next() {
                    return Some(RedisCacheAssetEither::<A, B>::B(d));
                }
                None
            }
        }
    }
}

pub enum RedisCachePageMerge<PA: Page, PB: Page> {
    A(PA),
    B(PB),
}

impl<PA: Page, PB: Page> Page for RedisCachePageMerge<PA, PB> {
    fn name(&self) -> &str {
        match self {
            Self::A(v) => v.name(),
            Self::B(v) => v.name(),
        }
    }

    fn branch(&self) -> &str {
        match self {
            Self::A(v) => v.branch(),
            Self::B(v) => v.branch(),
        }
    }

    fn owner(&self) -> &str {
        match self {
            Self::A(v) => v.owner(),
            Self::B(v) => v.owner(),
        }
    }

    fn version(&self) -> &str {
        match self {
            Self::A(v) => v.version(),
            Self::B(v) => v.version(),
        }
    }
}

impl<PA: Page, PB: Page> AssetQueryable for RedisCachePageMerge<PA, PB> {
    async fn asset_at(&self, path: &std::path::Path) -> Result<impl Asset, AssetError> {
        match self {
            Self::A(v) => match v.asset_at(path).await {
                Ok(v) => Ok(RedisCacheAssetEither::A(v)),
                Err(e) => Err(e),
            },
            Self::B(v) => match v.asset_at(path).await {
                Ok(v) => Ok(RedisCacheAssetEither::B(v)),
                Err(e) => Err(e),
            },
        }
    }

    fn assets(&self) -> Result<impl Iterator<Item = impl Asset>, AssetError> {
        match self {
            Self::A(v) => match v.assets() {
                Ok(v) => Ok(RedisCacheAssetIterEither::A(v)),
                Err(e) => Err(e),
            },
            Self::B(v) => match v.assets() {
                Ok(v) => Ok(RedisCacheAssetIterEither::B(v)),
                Err(e) => Err(e),
            },
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
            "page:{}:{}:{}:asset:{}",
            self.owner(),
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
                        // TODO: Error reporting
                        let _ = conn
                            .set::<String, &str, String>(key.clone(), v.body())
                            .await;
                        if self.ttl.is_some() {
                            let _ = conn.expire::<String, String>(key, self.ttl.unwrap()).await;
                        }
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
    ttl: Option<i64>,
}

impl<PS: PageSource> PageSource for RedisCacheSource<PS> {
    async fn page_at(
        &self,
        owner: String,
        name: String,
        branch: String,
    ) -> Result<impl Page, crate::page::PageError> {
        match self.upstream.page_at(owner, name, branch).await {
            Ok(page) => Ok({
                let mut conn = match self.client.get_multiplexed_async_connection().await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Failed to create multiplexed async Redis connection: {}", e);
                        return Err(PageError::ProviderError);
                    }
                };
                let version_key = format!(
                    "page:{}:{}:{}:version",
                    page.owner(),
                    page.name(),
                    page.branch()
                );
                match conn.get::<String, String>(version_key.clone()).await {
                    Ok(v) => {
                        if v != page.version() {
                            // Invalidate cache
                            info!("Page was updated; Invalidating cache...");
                            let _ = conn
                                .del::<String, u32>(format!(
                                    "page:{}:{}:{}:*",
                                    page.owner(),
                                    page.name(),
                                    page.branch()
                                ))
                                .await;

                            let _ = conn
                                .set::<String, String, String>(
                                    version_key,
                                    page.version().to_string(),
                                )
                                .await;
                        }
                    }
                    Err(e) => {
                        let _ = conn
                            .set::<String, String, String>(version_key, page.version().to_string())
                            .await;
                    }
                }
                RedisCachePage {
                    upstream: page,
                    client: self.client.clone(),
                    ttl: self.ttl,
                }
            }),
            Err(e) => Err(e),
        }
    }

    async fn pages(
        &self,
    ) -> Result<impl Iterator<Item = impl crate::page::Page>, crate::page::PageError> {
        self.upstream.pages().await
    }

    async fn find_by_domains(&self, domains: &[&str]) -> Result<impl Page, crate::page::PageError> {
        debug!("Connecting to Redis to cache search...");
        let mut conn = match self.client.get_multiplexed_async_connection().await {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to create multiplexed async Redis connection: {}", e);
                return Err(PageError::ProviderError);
            }
        };
        for domain in domains {
            let key_o = format!("domain:owner:{}", domain);
            let key_r = format!("domain:name:{}", domain);
            if let Ok(o) = conn.get::<String, String>(key_o).await {
                if let Ok(r) = conn.get::<String, String>(key_r).await {
                    if let Ok(upstream) =
                        self.page_at(o, r, self.default_branch().to_string()).await
                    {
                        info!("Cache hit! Found by cached domain.");
                        return Ok(RedisCachePage {
                            upstream: RedisCachePageMerge::A(upstream),
                            client: self.client.clone(),
                            ttl: self.ttl,
                        });
                    }
                }
            }
        }
        info!("Cache miss! Finding by domain...");

        let find = self.upstream.find_by_domains(domains).await;
        match find {
            Ok(page) => {
                for domain in domains {
                    let key_o = format!("domain:{}:owner", domain);
                    let key_r = format!("domain:{}:name", domain);
                    // TODO: Error reporting
                    let _ = conn
                        .set::<String, String, String>(key_o, page.owner().to_string())
                        .await;
                    let _ = conn
                        .set::<String, String, String>(key_r, page.name().to_string())
                        .await;
                }

                return Ok(RedisCachePage {
                    upstream: RedisCachePageMerge::B(page),
                    client: self.client.clone(),
                    ttl: self.ttl,
                });
            }
            Err(e) => Err(e),
        }
    }
}
