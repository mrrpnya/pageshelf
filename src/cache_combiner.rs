use std::sync::Arc;

use color_eyre::{
    Section,
    eyre::{self, Context},
};
use config::{Config, ConfigError};
#[cfg(feature = "redis")]
use pageshelf_provider_cache_redis::RedisCache;
use tracing::{Level, info, span};

use pageshelf_core::cache::{Cache, CacheConnection, CacheError};

// TODO: Make this more easily extensible?

/* -------------------------------------------------------------------------- */
/*                                Configuration                               */
/* -------------------------------------------------------------------------- */

const CONFIG_CACHE_KEY: &str = "cache";

pub fn cache_from_config(config: &Config) -> Result<Option<CacheCombinator>, eyre::Report> {
    let span = span!(Level::INFO, "cache_from_config");
    let _span_guard = span.enter();

    // Scope it down to the CONFIG_CACHE_KEY
    let cache_conf_map = match config.get_table(CONFIG_CACHE_KEY) {
        Ok(v) => v,
        Err(ConfigError::NotFound(_)) => {
            info!("No cache section found in the config, skipping cache setup");
            return Ok(None);
        }
        Err(e) => {
            return Err(eyre::Report::from(e));
        }
    };

    let mut cache_config = Config::builder();

    for (key, value) in cache_conf_map {
        cache_config = cache_config.set_override(key, value).unwrap();
    }

    let cache_config = cache_config
        .build()
        .wrap_err("Failed to create scoped cache configuration")?;

    let enabled = cache_config.get_bool("enabled").unwrap_or(false);

    if !enabled {
        info!("Cache is not enabled, skipping cache setup");
        return Ok(None);
    }

    // Deserialize Configuration (to identify basic details, like what cache to try)
    let backend = cache_config
        .get_string("backend")
        .wrap_err("Unable to find a backend from the cache config")
        .suggestion("Check your configuration; Ensure `cache.backend` exists?")
        .map(|f| f.to_lowercase())?;

    info!("Cache backend is determined to be \"{backend}\"");

    // Use configuration and pass cache_config (in its entirety) to appropriate cache constructors
    match backend.as_str() {
        "redis" => {
            #[cfg(not(feature = "redis"))]
            {
                error!("Cache backend \"{backend}\" is not available");
                info!("Build with feature \"redis\" for availability");
            }
            #[cfg(feature = "redis")]
            {
                RedisCache::from_config(&cache_config).map(|c| {
                    info!("Redis cache set up");
                    Some(CacheCombinator::Redis(c))
                })
            }
        }
        "valkey" => {
            #[cfg(not(feature = "redis"))]
            {
                error!("Cache backend \"{backend}\" is not available");
                info!("Build with feature \"redis\" for availability");
            }
            #[cfg(feature = "redis")]
            {
                RedisCache::from_config(&cache_config).map(|c| {
                    info!("Redis cache set up (may be connecting to Valkey instead)");
                    Some(CacheCombinator::Redis(c))
                })
            }
        }
        _ => {
            // TODO: The 'available' providers and the checked-for providers are not the same (could foreseeably cause issues)
            let available = [
                #[cfg(feature = "redis")]
                "redis",
                "valkey",
            ];
            Err(eyre::Report::msg(format!("Cache backend \"{backend}\" is not available"))
            .note(format!("Available backends: [{}]", available.join(", ")))
            .with_suggestion(|| {
                #[allow(clippy::const_is_empty)]
                if available.is_empty() {
                    "Rebuild with a cache provider enabled (which can be done via feature flags)"
                } else {
                    "Check cache.backend in your config and ensure it is set to an available backend"
                }
            }
            ))
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                            Multitype Connection                            */
/* -------------------------------------------------------------------------- */

#[derive(Clone)]
pub enum CacheCombinator {
    #[cfg(feature = "redis")]
    Redis(RedisCache),
}

pub enum CacheConnectionCombinator<'a> {
    #[cfg(feature = "redis")]
    Redis(<RedisCache as Cache>::Connection<'a>),
}

impl Cache for CacheCombinator {
    type Connection<'a> = CacheConnectionCombinator<'a>;

    async fn connect<'a>(&'a self) -> Result<Self::Connection<'a>, CacheError> {
        match self {
            #[cfg(feature = "redis")]
            Self::Redis(redis) => redis.connect().await.map(CacheConnectionCombinator::Redis),
        }
    }
}

impl<'a> CacheConnection for CacheConnectionCombinator<'a> {
    async fn set_asset(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &std::path::Path,
        asset: &[u8],
    ) -> Result<(), CacheError> {
        match self {
            Self::Redis(r) => r.set_asset(owner, project, channel, path, asset).await,
        }
    }

    async fn delete_asset(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &std::path::Path,
    ) -> Result<(), CacheError> {
        match self {
            Self::Redis(r) => r.delete_asset(owner, project, channel, path).await,
        }
    }

    async fn delete_page(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<(), CacheError> {
        match self {
            Self::Redis(r) => r.delete_page(owner, project, channel).await,
        }
    }

    async fn get_asset(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &std::path::Path,
    ) -> Result<Arc<[u8]>, CacheError> {
        match self {
            Self::Redis(r) => r.get_asset(owner, project, channel, path).await,
        }
    }

    async fn get_first_asset(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &[&std::path::Path],
    ) -> Result<(usize, Arc<[u8]>), CacheError> {
        match self {
            Self::Redis(r) => r.get_first_asset(owner, project, channel, path).await,
        }
    }

    async fn get_page_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<std::sync::Arc<[u8]>, CacheError> {
        match self {
            Self::Redis(r) => r.get_page_version(owner, project, channel).await,
        }
    }

    async fn purge(&mut self) -> Result<Option<u32>, CacheError> {
        match self {
            Self::Redis(r) => r.purge().await,
        }
    }

    async fn set_page_version(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
        version: &[u8],
    ) -> Result<(), CacheError> {
        match self {
            Self::Redis(r) => r.set_page_version(owner, project, channel, version).await,
        }
    }

    async fn test(&mut self) -> Result<String, eyre::Report> {
        match self {
            Self::Redis(r) => r.test().await,
        }
    }
}
