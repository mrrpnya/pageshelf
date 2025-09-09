/// A Layer that allows using Caches to temporarily store page info and Assets.
use std::sync::Arc;

use log::{debug, error, info};

use crate::{
    Asset, AssetError, AssetSource, Cache, CacheConnection, Page, PageError, PageSource,
    PageSourceLayer,
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

impl<PS: PageSource, C: Cache> PageSourceLayer<PS> for CacheLayer<C> {
    type Source = CacheLayerSource<PS, C>;

    fn wrap(&self, page_source: PS) -> Self::Source {
        Self::Source {
            upstream: page_source,
            cache: self.cache.clone(),
        }
    }
}

pub struct CachePage<P: Page, C: Cache> {
    upstream: P,
    cache: Arc<C>,
}

impl<P: Page, C: Cache> Page for CachePage<P, C> {
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

pub enum CacheAsset<A: Asset> {
    Hold(Vec<u8>),
    Load(A),
}

impl<A: Asset> Asset for CacheAsset<A> {
    fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Hold(data) => data,
            Self::Load(asset) => asset.into_bytes(),
        }
    }
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Hold(data) => data,
            Self::Load(asset) => asset.bytes(),
        }
    }
}

pub enum CacheAssetEither<A: Asset, B: Asset> {
    A(A),
    B(B),
}

impl<A: Asset, B: Asset> Asset for CacheAssetEither<A, B> {
    fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::A(data) => data.into_bytes(),
            Self::B(data) => data.into_bytes(),
        }
    }
    fn bytes(&self) -> &[u8] {
        match self {
            Self::A(data) => data.bytes(),
            Self::B(data) => data.bytes(),
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
    type Item = CacheAssetEither<A, B>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::A(data) => {
                if let Some(d) = data.next() {
                    return Some(CacheAssetEither::<A, B>::A(d));
                }
                None
            }
            Self::B(data) => {
                if let Some(d) = data.next() {
                    return Some(CacheAssetEither::<A, B>::B(d));
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

impl<PA: Page, PB: Page> AssetSource for RedisCachePageMerge<PA, PB> {
    async fn get_asset(&self, path: &std::path::Path) -> Result<impl Asset, AssetError> {
        match self {
            Self::A(v) => match v.get_asset(path).await {
                Ok(v) => Ok(CacheAssetEither::A(v)),
                Err(e) => Err(e),
            },
            Self::B(v) => match v.get_asset(path).await {
                Ok(v) => Ok(CacheAssetEither::B(v)),
                Err(e) => Err(e),
            },
        }
    }
}

impl<P: Page, C: Cache> AssetSource for CachePage<P, C> {
    async fn get_asset(&self, path: &std::path::Path) -> Result<impl Asset, AssetError> {
        let mut conn = match self.cache.connect().await {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to create cache connection: {:?}", e);
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
        match conn.get(&key).await {
            Ok(v) => {
                info!("Cache hit: {:?}", path);
                Ok(CacheAsset::Hold(v))
            }
            Err(e) => {
                info!("Cache miss (loading from upstream): {:?}", e);
                match self.upstream.get_asset(path).await {
                    Ok(v) => {
                        let _ = conn.set(&key, v.bytes()).await;
                        Ok(CacheAsset::Load(v))
                    }
                    Err(e) => {
                        error!("Error getting asset from upstream: {:?}", e);
                        Err(e)
                    }
                }
            }
        }
    }
}

pub struct CacheLayerSource<PS: PageSource, C: Cache> {
    upstream: PS,
    cache: Arc<C>,
}

impl<PS: PageSource, C: Cache> PageSource for CacheLayerSource<PS, C> {
    async fn page_at(
        &self,
        owner: String,
        name: String,
        branch: String,
    ) -> Result<impl Page, PageError> {
        let mut conn = match self.cache.connect().await {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to create cache connection: {:?}", e);
                return Err(PageError::ProviderError);
            }
        };
        match self.upstream.page_at(owner, name, branch).await {
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
                            return Err(PageError::ProviderError);
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
    }

    async fn pages(&self) -> Result<impl Iterator<Item = impl Page>, PageError> {
        self.upstream.pages().await
    }

    async fn find_by_domains(&self, domains: &[&str]) -> Result<impl Page, PageError> {
        debug!("Connecting to Redis to cache search...");
        let mut conn = match self.cache.connect().await {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to create cache connection: {:?}", e);
                return Err(PageError::ProviderError);
            }
        };
        for domain in domains {
            let key_o = format!("domain:owner:{}", domain);
            let key_r = format!("domain:name:{}", domain);
            if let Ok(o) = conn.get_string(&key_o).await {
                if let Ok(r) = conn.get_string(&key_r).await {
                    if let Ok(upstream) =
                        self.page_at(o, r, self.default_branch().to_string()).await
                    {
                        info!("Cache hit! Found by cached domain.");
                        return Ok(CachePage {
                            upstream: RedisCachePageMerge::A(upstream),
                            cache: self.cache.clone(),
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
                    let _ = conn.set(&key_o, page.owner().as_bytes()).await;
                    let _ = conn.set(&key_r, page.name().as_bytes()).await;
                }

                Ok(CachePage {
                    upstream: RedisCachePageMerge::B(page),
                    cache: self.cache.clone(),
                })
            }
            Err(e) => Err(e),
        }
    }
}
