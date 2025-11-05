use crate::{
    Asset, AssetError, AssetSource, Cache, CacheConnection, CacheError, ext::Normalizable,
    project::Page,
};
use lz4::block::{CompressionMode, compress, decompress};
use std::{path::Path, sync::Arc, time::Instant};
use tokio::task;
use tracing::{debug, error, info, warn};

const COMPRESSION_MODE: CompressionMode = CompressionMode::FAST(7);

pub struct CachePage<P: Page, C: Cache> {
    pub upstream: P,
    pub asset_key: String,
    pub cache: Arc<C>,
}
impl<P: Page, C: Cache + Send + Sync + 'static> CachePage<P, C> {
    pub fn new(upstream: P, cache: Arc<C>, owner: String, project: &str) -> Self {
        let asset_key = format!("p:{}:{}:{}:a", owner, project, upstream.name());
        let meta_key = format!("p:{}:{}:{}:meta", owner, project, upstream.name());
        let p = Self {
            upstream,
            cache: cache.clone(),
            asset_key: asset_key.clone(),
        };

        // Spawn background async task for version check
        let c = p.cache.clone();
        let ver = p.upstream.version().to_string();
        tokio::task::spawn_local(async move {
            Self::version_check(c, ver, asset_key, meta_key).await;
        });

        p
    }

    async fn version_check(
        cache: Arc<C>,
        expected_ver: String,
        asset_key: String,
        meta_key: String,
    ) {
        const CACHE_VERSION_KEY: &str = "version";

        info!("Checking cached version...");
        if let Ok(mut conn) = cache.connect().await {
            match conn.hget_string(&meta_key, CACHE_VERSION_KEY).await {
                Ok(cached_ver) => {
                    if cached_ver != expected_ver {
                        debug!(
                            cached.version = cached_ver,
                            upstream.version = expected_ver,
                            "Cache out of date; Purging page data"
                        );
                        let _ = conn.delete(&asset_key).await;
                        let _ = conn.delete(&meta_key).await;
                    } else {
                        debug!(cached.version = cached_ver, "Cache up to date")
                    }
                }
                Err(CacheError::NotFound) => {
                    let _ = conn
                        .hset(&meta_key, CACHE_VERSION_KEY, expected_ver.as_bytes())
                        .await;
                }
                Err(e) => {
                    error!("Error checking cache version: {e}")
                }
            }
        }
    }
}
impl<P: Page, C: Cache> crate::project::Page for CachePage<P, C> {
    fn name(&self) -> &str {
        self.upstream.name()
    }

    fn version(&self) -> &str {
        self.upstream.version()
    }

    async fn domains(&self) -> Result<impl Iterator<Item = String>, AssetError> {
        self.upstream.domains().await
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

pub enum CacheEither<A, B> {
    A(A),
    B(B),
}

// Only if A-B is asset
impl<A: Asset, B: Asset> Asset for CacheEither<A, B> {
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

impl<Item, AI: Iterator<Item = Item>, BI: Iterator<Item = Item>> Iterator for CacheEither<AI, BI> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::A(v) => v.next(),
            Self::B(v) => v.next(),
        }
    }
}

pub enum RedisCacheIterEither<A, B, AI: Iterator<Item = A>, BI: Iterator<Item = B>> {
    A(AI),
    B(BI),
}

impl<A, B, AI: Iterator<Item = A>, BI: Iterator<Item = B>> Iterator
    for RedisCacheIterEither<A, B, AI, BI>
{
    type Item = CacheEither<A, B>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::A(data) => {
                if let Some(d) = data.next() {
                    return Some(CacheEither::<A, B>::A(d));
                }
                None
            }
            Self::B(data) => {
                if let Some(d) = data.next() {
                    return Some(CacheEither::<A, B>::B(d));
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

pub enum RedisCachePageMergeDomainsMerge<A: Iterator<Item = String>, B: Iterator<Item = String>> {
    A(A),
    B(B),
}

impl<A: Iterator<Item = String>, B: Iterator<Item = String>> Iterator
    for RedisCachePageMergeDomainsMerge<A, B>
{
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::A(v) => v.next(),
            Self::B(v) => v.next(),
        }
    }
}

impl<PA: Page, PB: Page> Page for RedisCachePageMerge<PA, PB> {
    fn name(&self) -> &str {
        match self {
            Self::A(v) => v.name(),
            Self::B(v) => v.name(),
        }
    }

    fn version(&self) -> &str {
        match self {
            Self::A(v) => v.version(),
            Self::B(v) => v.version(),
        }
    }

    async fn domains(&self) -> Result<impl Iterator<Item = String>, AssetError> {
        match self {
            Self::A(v) => match v.domains().await {
                Ok(v) => Ok(RedisCachePageMergeDomainsMerge::A(v)),
                Err(e) => Err(e),
            },
            Self::B(v) => match v.domains().await {
                Ok(v) => Ok(RedisCachePageMergeDomainsMerge::B(v)),
                Err(e) => Err(e),
            },
        }
    }
}

impl<PA: Page, PB: Page> AssetSource for RedisCachePageMerge<PA, PB> {
    async fn get_asset(&self, path: &std::path::Path) -> Result<impl Asset, AssetError> {
        match self {
            Self::A(v) => match v.get_asset(path).await {
                Ok(v) => Ok(CacheEither::A(v)),
                Err(e) => Err(e),
            },
            Self::B(v) => match v.get_asset(path).await {
                Ok(v) => Ok(CacheEither::B(v)),
                Err(e) => Err(e),
            },
        }
    }

    async fn asset_keys(&self) -> Result<impl Iterator<Item = String>, AssetError> {
        match self {
            Self::A(v) => match v.asset_keys().await {
                Ok(v) => Ok(CacheEither::A(v)),
                Err(e) => Err(e),
            },
            Self::B(v) => match v.asset_keys().await {
                Ok(v) => Ok(CacheEither::B(v)),
                Err(e) => Err(e),
            },
        }
    }
}

impl<P: Page, C: Cache> AssetSource for CachePage<P, C> {
    async fn get_asset(&self, path: &std::path::Path) -> Result<impl Asset, AssetError> {
        let begin = Instant::now();
        let normalized = path.normalized();

        let mut conn = self.cache.connect().await.map_err(|e| {
            error!(error = ?e, "Failed to create cache connection");
            AssetError::ProviderError
        })?;

        let path_str = normalized.to_str().ok_or_else(|| {
            error!(?path, "Invalid UTF-8 in asset path");
            AssetError::ProviderError
        })?;

        match conn.hget(&self.asset_key, path_str).await {
            Ok(cached_bytes) => {
                // Decompress in blocking thread
                let decompressed = task::spawn_blocking(move || decompress(&cached_bytes, None))
                    .await
                    .map_err(|_| AssetError::ProviderError)?
                    .map_err(|e| {
                        error!(error = ?e, ?path, "Failed to decompress cached asset");
                        AssetError::ProviderError
                    })?;

                info!(?path, "[{}us] Cache hit", begin.elapsed().as_micros());
                Ok(CacheAsset::Hold(decompressed))
            }
            Err(_) => {
                debug!(?path, "Cache miss, loading from upstream");

                let asset = self.upstream.get_asset(path).await?;
                let bytes = asset.bytes().to_vec();

                // Compress in blocking thread
                let compressed =
                    task::spawn_blocking(move || compress(&bytes, Some(COMPRESSION_MODE), true))
                        .await
                        .map_err(|_| AssetError::ProviderError)?
                        .map_err(|e| {
                            error!(error = ?e, "Failed to compress asset for caching");
                            AssetError::ProviderError
                        })?;

                // Store compressed bytes in cache
                if let Err(e) = conn.hset(&self.asset_key, path_str, &compressed).await {
                    warn!(error = ?e, %self.asset_key, "Failed to cache asset");
                }

                Ok(CacheAsset::Load(asset))
            }
        }
    }

    async fn get_first_asset_in(&self, paths: &[&Path]) -> Result<(usize, impl Asset), AssetError> {
        let begin = Instant::now();
        let mut conn = self.cache.connect().await.map_err(|e| {
            error!(error = ?e, "Failed to create cache connection");
            AssetError::ProviderError
        })?;

        // Collect UTF-8 valid paths
        let fields: Vec<_> = paths
            .iter()
            .map(|p| p.to_str().ok_or(AssetError::ProviderError))
            .collect::<Result<_, _>>()?;

        match conn.hget_first_field(&self.asset_key, &fields).await {
            Ok((idx, cached_bytes)) => {
                let decompressed = task::spawn_blocking(move || decompress(&cached_bytes, None))
                    .await
                    .map_err(|_| AssetError::ProviderError)?
                    .map_err(|e| {
                        error!(error = ?e, "Failed to decompress cached asset");
                        AssetError::ProviderError
                    })?;

                info!(
                    path = ?paths[idx],
                    "[{}us] Cache hit (first available)",
                    begin.elapsed().as_micros()
                );
                Ok((idx, CacheAsset::Hold(decompressed)))
            }
            Err(CacheError::NotFound) => {
                debug!("No assets found in cache, fetching from upstream");

                for (idx, path) in paths.iter().enumerate() {
                    match self.upstream.get_asset(path).await {
                        Ok(asset) => {
                            let bytes = asset.bytes().to_vec();

                            let compressed = task::spawn_blocking(move || {
                                compress(&bytes, Some(COMPRESSION_MODE), true)
                            })
                            .await
                            .map_err(|_| AssetError::ProviderError)?
                            .map_err(|e| {
                                error!(error = ?e, "Failed to compress asset for caching");
                                AssetError::ProviderError
                            })?;

                            if let Err(e) =
                                conn.hset(&self.asset_key, fields[idx], &compressed).await
                            {
                                warn!(error = ?e, %self.asset_key, "Failed to cache asset");
                            }

                            info!(
                                ?path,
                                "[{}us] Loaded from upstream",
                                begin.elapsed().as_micros()
                            );
                            return Ok((idx, CacheAsset::Load(asset)));
                        }
                        Err(AssetError::NotFound) => continue,
                        Err(e) => {
                            error!(error = ?e, ?path, "Error getting asset from upstream");
                            return Err(e);
                        }
                    }
                }

                Err(AssetError::NotFound)
            }
            Err(e) => {
                error!(error = ?e, "Error checking multiple cache keys");
                Err(AssetError::ProviderError)
            }
        }
    }

    async fn asset_keys(&self) -> Result<impl Iterator<Item = String>, AssetError> {
        self.upstream.asset_keys().await
    }
}
