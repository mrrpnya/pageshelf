#[cfg(feature = "redis")]
mod redis;
use color_eyre::{
    Section,
    eyre::{self, Context},
};
use config::{Config, ConfigError};
#[cfg(feature = "redis")]
pub use redis::*;
use tracing::{Level, info, span};

use crate::{Cache, CacheConnection};

// TODO: Make this more easily extensible?

/// Where to find the cache configuration within the config
const CONFIG_CACHE_KEY: &str = "cache";

pub struct Configuration {}

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

    async fn connect<'a>(&'a self) -> Result<Self::Connection<'a>, crate::CacheError> {
        match self {
            #[cfg(feature = "redis")]
            Self::Redis(redis) => redis.connect().await.map(CacheConnectionCombinator::Redis),
        }
    }
}

impl<'a> CacheConnection for CacheConnectionCombinator<'a> {
    async fn hset(
        &mut self,
        key: &str,
        field: &str,
        value: &[u8],
    ) -> Result<(), crate::CacheError> {
        match self {
            #[cfg(feature = "redis")]
            Self::Redis(conn) => conn.hset(key, field, value).await,
        }
    }

    async fn hget_first_field(
        &mut self,
        key: &str,
        fields: &[&str],
    ) -> Result<(usize, Vec<u8>), crate::CacheError> {
        match self {
            #[cfg(feature = "redis")]
            Self::Redis(conn) => conn.hget_first_field(key, fields).await,
        }
    }

    async fn hget_string(&mut self, key: &str, field: &str) -> Result<String, crate::CacheError> {
        match self {
            #[cfg(feature = "redis")]
            Self::Redis(conn) => conn.hget_string(key, field).await,
        }
    }

    async fn hget(&mut self, key: &str, field: &str) -> Result<Vec<u8>, crate::CacheError> {
        match self {
            #[cfg(feature = "redis")]
            Self::Redis(conn) => conn.hget(key, field).await,
        }
    }

    async fn delete(&mut self, key: &str) -> Result<u32, crate::CacheError> {
        match self {
            #[cfg(feature = "redis")]
            Self::Redis(conn) => conn.delete(key).await,
        }
    }

    async fn purge(&mut self) -> Result<Option<u32>, crate::CacheError> {
        match self {
            Self::Redis(conn) => conn.purge().await,
        }
    }
}

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
