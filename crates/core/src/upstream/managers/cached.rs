use std::sync::Arc;

use metrics::counter;
use tracing::{info, warn};

use crate::{
    cache::{Cache, CacheConnection, CacheError},
    event::{Event, EventBus},
    upstream::{
        Upstream, UpstreamError,
        source::{
            AssetListSource, AssetSource, PageListComponentsSource, PageListSource,
            PageVersionSource,
        },
    },
};

/* -------------------------------------------------------------------------- */
/*                               Implementation                               */
/* -------------------------------------------------------------------------- */

/// A caching wrapper around a source provider.
///
/// `CachedUpstream` combines a [Cache] implementation with a provider
/// to reduce redundant network or storage requests. When fetching assets, it first
/// checks the cache for a matching version. If it is available,the cached asset
/// is returned. Otherwise, it fetches from the upstream, updates the cache,
/// and returns the new data.
pub struct CachedSource<U: AssetSource + PageVersionSource + 'static, C: Cache + 'static> {
    cache: C,
    provider: U,
    event_bus: EventBus,
}

impl<U: AssetSource + PageVersionSource + 'static, C: Cache + 'static> CachedSource<U, C> {
    /// Creates a new [CachedUpstream].
    pub fn wrap(cache: C, provider: U, event_bus: EventBus) -> Arc<Self> {
        let s = Arc::new(Self {
            cache,
            provider,
            event_bus,
        });

        // Now we can clone Arc for the background task
        s.enable_event_invalidation();

        s
    }

    fn enable_event_invalidation(self: &Arc<Self>) {
        let bus = self.event_bus.clone();
        let cached_self = Arc::clone(self);

        tokio::spawn(async move {
            bus.subscribe(move |event| {
                let cached_self = Arc::clone(&cached_self);
                tokio::spawn(async move {
                    match event {
                        Event::PageRemoved {
                            owner,
                            project,
                            channel,
                        } => {
                            let mut conn = match cached_self.cache.connect().await {
                                Ok(c) => c,
                                Err(e) => {
                                    warn!("Failed to connect to cache: {e}");
                                    return;
                                }
                            };
                            let _ = conn.delete_page(&owner, &project, &channel).await;
                        }
                        Event::PageDiscovered {
                            owner,
                            project,
                            channel,
                        } => {
                            let mut conn = match cached_self.cache.connect().await {
                                Ok(c) => c,
                                Err(e) => {
                                    warn!("Failed to connect to cache: {e}");
                                    return;
                                }
                            };

                            if let Ok(upstream_ver) = cached_self
                                .provider
                                .get_page_version(&owner, &project, &channel)
                                .await
                            {
                                match conn.get_page_version(&owner, &project, &channel).await {
                                    Ok(cached_ver) => {
                                        if &*cached_ver != upstream_ver.as_bytes() {
                                            let _ =
                                                conn.delete_page(&owner, &project, &channel).await;
                                            let _ = conn
                                                .set_page_version(
                                                    &owner,
                                                    &project,
                                                    &channel,
                                                    upstream_ver.as_bytes(),
                                                )
                                                .await;
                                        }
                                    }
                                    Err(_) => {
                                        let _ = conn.delete_page(&owner, &project, &channel).await;
                                        let _ = conn
                                            .set_page_version(
                                                &owner,
                                                &project,
                                                &channel,
                                                upstream_ver.as_bytes(),
                                            )
                                            .await;
                                    }
                                }
                            };
                        }
                    }
                });
            });
        });
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Sources                                  */
/* -------------------------------------------------------------------------- */

impl<U: Upstream, C: Cache + 'static> Upstream for CachedSource<U, C> {}

impl<U: PageVersionSource + AssetSource, C: Cache + 'static> CachedSource<U, C> {
    /// Gives access to the cache that is in use.
    pub fn cache(&self) -> &C {
        &self.cache
    }
}

impl<U: PageVersionSource + AssetSource + PageListComponentsSource, C: Cache + 'static>
    PageListComponentsSource for CachedSource<U, C>
{
    async fn list_owners(&self) -> Result<Arc<[String]>, UpstreamError> {
        self.provider.list_owners().await
    }
    async fn list_projects(&self, owner: &str) -> Result<Arc<[String]>, UpstreamError> {
        self.provider.list_projects(owner).await
    }
    async fn list_channels(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Arc<[String]>, UpstreamError> {
        self.provider.list_channels(owner, project).await
    }

    async fn has_page(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<bool, UpstreamError> {
        match self.list_channels(owner, project).await {
            Ok(chans) => Ok(chans.iter().any(|c| c == channel)),
            Err(UpstreamError::NotImplemented) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

impl<U: AssetSource + PageVersionSource, C: Cache + 'static> PageVersionSource
    for CachedSource<U, C>
{
    async fn get_page_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<String, UpstreamError> {
        self.provider
            .get_page_version(owner, project, channel)
            .await
    }
}

impl<U: AssetSource + PageVersionSource, C: Cache + 'static> AssetSource for CachedSource<U, C> {
    async fn get_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &std::path::Path,
    ) -> Result<Arc<[u8]>, UpstreamError> {
        info!("{owner}:{project}:{channel}");

        let mut conn = match self.cache.connect().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to connect to cache: {e}");
                counter!("asset.cache.error").increment(1);
                return self
                    .provider
                    .get_asset_bytes(owner, project, channel, path)
                    .await;
            }
        };

        match conn.get_asset(owner, project, channel, path).await {
            Ok(data) => {
                tracing::debug!("Cache hit: {owner}:{project}:{channel}:{path:?}");
                counter!("asset.cache.hit").increment(1);

                return Ok(data);
            }
            Err(CacheError::NotFound) => {
                tracing::debug!("Cache miss: {owner}:{project}:{channel}:{path:?}");
                counter!("asset.cache.miss").increment(1);
            }
            Err(_) => {
                return Err(UpstreamError::ProviderError);
            }
        }

        match self
            .provider
            .get_asset_bytes(owner, project, channel, path)
            .await
        {
            Ok(data) => {
                tracing::debug!("Upstream hit: {owner}:{project}:{channel}:{path:?}");
                let _ = conn.set_asset(owner, project, channel, path, &data).await;
                counter!("asset.cache.upstream.hit").increment(1);

                Ok(data)
            }
            Err(UpstreamError::NotFound) => {
                counter!("asset.cache.upstream.miss").increment(1);
                Err(UpstreamError::NotFound)
            }
            Err(e) => Err(e),
        }
    }

    async fn get_first_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        paths: &[&std::path::Path],
    ) -> Result<(usize, Arc<[u8]>), UpstreamError> {
        let mut conn = match self.cache.connect().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to connect to cache: {e}");
                counter!("asset.cache.error").increment(1);
                return self
                    .provider
                    .get_first_asset_bytes(owner, project, channel, paths)
                    .await;
            }
        };

        match conn.get_first_asset(owner, project, channel, paths).await {
            Ok(data) => {
                let path = paths[data.0];
                tracing::debug!("Cache hit: {owner}:{project}:{channel}:{path:?}");
                counter!("asset.cache.hit").increment(1);

                return Ok(data);
            }
            Err(CacheError::NotFound) => {
                tracing::debug!("Cache miss: {owner}:{project}:{channel}:{paths:?}");
                counter!("asset.cache.miss").increment(1);
            }
            Err(_) => {
                return Err(UpstreamError::ProviderError);
            }
        }

        match self
            .provider
            .get_first_asset_bytes(owner, project, channel, paths)
            .await
        {
            Ok(data) => {
                let path = paths[data.0];
                tracing::debug!("Upstream hit: {owner}:{project}:{channel}:{path:?}");
                counter!("asset.cache.upstream.hit").increment(1);
                let _ = conn.set_asset(owner, project, channel, path, &data.1).await;
                Ok(data)
            }
            Err(UpstreamError::NotFound) => {
                counter!("asset.cache.upstream.miss").increment(1);
                Err(UpstreamError::NotFound)
            }
            Err(e) => Err(e),
        }
    }
}

impl<U: AssetSource + PageVersionSource + AssetListSource, C: Cache + 'static> AssetListSource
    for CachedSource<U, C>
{
    async fn list_assets(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<Arc<[String]>, UpstreamError> {
        self.provider.list_assets(owner, project, channel).await
    }
}

/* ---------------------------------- Pages --------------------------------- */

impl<U: AssetSource + PageVersionSource + PageListSource, C: Cache + 'static> PageListSource
    for CachedSource<U, C>
{
    async fn list_pages(
        &self,
    ) -> Result<Arc<(Arc<[String]>, Arc<[String]>, Arc<[String]>)>, UpstreamError> {
        self.provider.list_pages().await
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cache::MockCache, upstream::mock::MockUpstream};
    use std::path::PathBuf;

    const OWNER: &str = "owner";
    const PROJECT: &str = "project";
    const CHANNEL: &str = "channel";
    const PATH: &str = "file.txt";
    const DATA: &[u8] = b"hello world";

    fn path() -> PathBuf {
        PathBuf::from(PATH)
    }

    #[tokio::test]
    async fn returns_asset_from_cache_when_version_matches() {
        let cache = MockCache::default();
        let mut conn = cache.connect().await.unwrap();
        conn.set_page_version(OWNER, PROJECT, CHANNEL, b"v1")
            .await
            .unwrap();
        conn.set_asset(OWNER, PROJECT, CHANNEL, &path(), DATA)
            .await
            .unwrap();

        let provider = MockUpstream::default().with_asset(OWNER, PROJECT, CHANNEL, &path(), DATA);

        let cached_upstream = CachedSource::wrap(cache, provider, EventBus::default());

        let result = cached_upstream
            .get_asset_bytes(OWNER, PROJECT, CHANNEL, &path())
            .await
            .unwrap();

        assert_eq!(&*result, DATA);
    }

    #[tokio::test]
    async fn fetches_from_upstream_when_cache_is_empty() {
        let cache = MockCache::default();

        let provider = MockUpstream::default().with_asset(OWNER, PROJECT, CHANNEL, &path(), DATA);

        let cached_upstream = CachedSource::wrap(cache, provider, EventBus::default());

        let result = cached_upstream
            .get_asset_bytes(OWNER, PROJECT, CHANNEL, &path())
            .await
            .unwrap();

        assert_eq!(&*result, DATA);
    }

    #[tokio::test]
    async fn updates_cache_when_version_mismatches() {
        let cache = MockCache::default();
        let mut conn = cache.connect().await.unwrap();
        conn.set_page_version(OWNER, PROJECT, CHANNEL, b"old_version")
            .await
            .unwrap();
        conn.set_asset(OWNER, PROJECT, CHANNEL, &path(), b"old_data")
            .await
            .unwrap();

        let provider = MockUpstream::default().with_asset(OWNER, PROJECT, CHANNEL, &path(), DATA);

        let cached_upstream = CachedSource::wrap(cache, provider, EventBus::default());

        let result = cached_upstream
            .get_asset_bytes(OWNER, PROJECT, CHANNEL, &path())
            .await
            .unwrap();

        assert_eq!(&*result, DATA);

        let conn = cached_upstream.cache.connect().await.unwrap();
        let cached_data = conn
            .get_asset(OWNER, PROJECT, CHANNEL, &path())
            .await
            .unwrap();
        assert_eq!(&*cached_data, DATA);
    }

    #[tokio::test]
    async fn get_page_version_delegates_to_provider() {
        let cache = MockCache::default();
        let _conn = cache.connect().await.unwrap();

        let provider = MockUpstream::default().with_asset(OWNER, PROJECT, CHANNEL, &path(), DATA);

        let cached_upstream = CachedSource::wrap(cache, provider, EventBus::default());

        // Manually set a version in the cache for testing
        let mut conn = cached_upstream.cache.connect().await.unwrap();
        conn.set_page_version(OWNER, PROJECT, CHANNEL, b"v123")
            .await
            .unwrap();

        let version = cached_upstream
            .get_page_version(OWNER, PROJECT, CHANNEL)
            .await
            .unwrap();

        assert_eq!(version, "v123");
    }
}
