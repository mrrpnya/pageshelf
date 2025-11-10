//! A Cache that allows using Redis to cache page info and Assets.
//!
//! Redis is a high-speed in-memory cache with data durability, capable of sub-millisecond speeds.
//! It is useful for reducing queries upstream (especially when deployed off-site).
//! See <https://redis.io/> for more information about Redis itself.
#![warn(missing_docs)]
#![forbid(unsafe_code)]
use color_eyre::{
    Section,
    eyre::{self, Context},
};
use config::{Config, ConfigError};
use redis::AsyncCommands;
use redis::{Client, RedisError, Script, aio::MultiplexedConnection};
use std::sync::{Arc, LazyLock};
use tracing::{Level, error, info, span};

use pageshelf_core::cache::{Cache, CacheError, KVHashCacheConnection};

static REDIS_HGET_FIRST_FIELD_SCRIPT: LazyLock<Script> =
    LazyLock::new(|| Script::new(include_str!("scripts/hget_first_field.lua")));

/// Adapter for using Redis as a cache
#[derive(Clone)]
pub struct RedisCache {
    client: Arc<Client>,
    ttl: Option<u32>,
}

impl RedisCache {
    /// Sets a [RedisCache] up to connect to a specific Redis instance
    pub fn new(host: &str, port: u16, ttl: Option<u32>) -> Result<Self, RedisError> {
        let address = format!("redis://{}:{}", host, port);
        match redis::Client::open(address.clone()) {
            Ok(v) => Ok(Self {
                client: Arc::new(v),
                ttl,
            }),
            Err(e) => {
                error!(
                    redis.address = address,
                    "Failed to create Redis cache: {}", e
                );
                Err(e)
            }
        }
    }

    /// Automatically sets a [RedisCache] up based on a given configuration
    pub fn from_config(config: &Config) -> Result<Self, eyre::Report> {
        let span = span!(Level::INFO, "redis::from_config");
        let _span_guard = span.enter();
        let host = match config.get_string("host") {
            Ok(v) => v,
            Err(e) => {
                match e {
                    ConfigError::NotFound(_) => {
                        info!("No host specified; Assuming localhost");
                        "localhost".to_string()
                    }
                    e => {
                        return Err(eyre::Report::from(e)
                        .wrap_err("Failed to get host from config")
                        .suggestion("Check your configuration and ensure cache.host is correctly formed"));
                    }
                }
            }
        };
        let port: u16 = match config.get("port") {
            Ok(v) => v,
            Err(e) => match e {
                ConfigError::NotFound(_) => {
                    info!("No port specified; Assuming default Redis port");
                    6379u16
                }
                _ => {
                    return Err(eyre::Report::from(e)
                        .wrap_err("Failed to get port from config")
                        .suggestion("Check your configuration and ensure cache.port is correctly formed (and is a 16-bit integer)"));
                }
            },
        };
        let ttl: Option<u32> = match config.get::<u32>("ttl") {
            Ok(v) => Some(v),
            Err(e) => match e {
                ConfigError::NotFound(_) => {
                    info!("No cache TTL specified");
                    None
                }
                _ => {
                    return Err(eyre::Report::from(e)
                        .wrap_err("Failed to get cache TTL from config")
                        .suggestion(
                            "Check your configuration and ensure cache.ttl is correctly formed (and is a 32-bit integer)",
                        ));
                }
            },
        };

        Self::new(&host, port, ttl)
            .wrap_err("Failed to set up Redis cache")
            .suggestion("Check if Redis is online and reachable")
    }
}

impl Cache for RedisCache {
    type Connection<'a> = RedisCacheConnection;
    async fn connect<'a>(&'a self) -> Result<Self::Connection<'a>, crate::CacheError> {
        let conn = self.client.get_multiplexed_async_connection().await;

        let conn = match conn {
            Ok(v) => v,
            Err(e) => {
                error!("Redis error: {}", e);
                return Err(CacheError::ConnectionError);
            }
        };

        Ok(RedisCacheConnection {
            conn,
            ttl: self.ttl,
        })
    }
}

/// An active connection to a Redis cache.
pub struct RedisCacheConnection {
    conn: MultiplexedConnection,
    ttl: Option<u32>,
}

impl RedisCacheConnection {
    /// Create a connection adapter from an active Redis connection
    ///
    /// If provided, `ttl` can help specify the desired longevity of assets.
    pub fn new(conn: MultiplexedConnection, ttl: Option<u32>) -> Self {
        Self { conn, ttl }
    }
}

impl KVHashCacheConnection for RedisCacheConnection {
    async fn hset(&mut self, key: &str, field: &str, value: &[u8]) -> Result<(), CacheError> {
        let result = self.conn.hset(key, field, value).await;

        match result {
            Ok(()) => {}
            Err(e) => {
                error!("Redis error while setting key \"{}\"'s value: {}", key, e);
                return Err(CacheError::OperationError(e.to_string()));
            }
        }

        if let Some(ttl) = self.ttl {
            let result = self.conn.expire(key, i64::from(ttl)).await;

            match result {
                Ok(()) => {}
                Err(e) => {
                    error!(
                        "Redis error while setting key \"{}\"'s expiration: {}",
                        key, e
                    );
                    return Err(CacheError::OperationError(e.to_string()));
                }
            }
        }

        Ok(())
    }

    async fn hget(&self, key: &str, field: &str) -> Result<Arc<[u8]>, CacheError> {
        let mut conn = self.conn.clone();
        // Redis
        let exists = conn.hexists::<&str, &str, bool>(key, field).await;

        match exists {
            Ok(v) => {
                if !v {
                    return Err(CacheError::NotFound);
                }
            }
            Err(e) => {
                error!(
                    "Redis error while checking if key \"{}\" exists: {}",
                    key, e
                );
                return Err(CacheError::OperationError(e.to_string()));
            }
        }

        let result = conn.hget::<&str, &str, Arc<[u8]>>(key, field).await;

        match result {
            Ok(v) => Ok(v),
            Err(e) => {
                error!("Redis error while getting key \"{}\": {}", key, e);
                Err(CacheError::OperationError(e.to_string()))
            }
        }
    }

    /// Finds the first available field in the hashset
    async fn hget_first_field(
        &self,
        key: &str,
        fields: &[&str],
    ) -> Result<(usize, Arc<[u8]>), CacheError> {
        let mut conn = self.conn.clone();

        let res: Option<(usize, Arc<[u8]>)> = REDIS_HGET_FIRST_FIELD_SCRIPT
            .arg(fields)
            .key(key)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| CacheError::OperationError(e.to_string()))?;

        match res {
            Some((idx, value)) => Ok((idx, value)),
            None => Err(CacheError::NotFound),
        }
    }

    async fn hdel(&mut self, key: &str) -> Result<Option<u32>, CacheError> {
        let keys: Vec<String> = match self.conn.keys::<&str, Vec<String>>(key).await {
            Ok(k) => k,
            Err(e) => {
                error!("Redis error while listing keys: {}", e);
                return Err(CacheError::OperationError(e.to_string()));
            }
        };
        let mut count: u32 = 0;
        for key in keys {
            let result = self.conn.del::<&str, u32>(&key).await;

            match result {
                Ok(v) => count += v,
                Err(e) => {
                    error!("Redis error while deleting key \"{}\": {}", key, e);
                    return Err(CacheError::OperationError(e.to_string()));
                }
            }
        }
        Ok(Some(count))
    }

    async fn hdel_field(&mut self, key: &str, field: &str) -> Result<Option<u32>, CacheError> {
        let keys: Vec<String> = match self.conn.keys::<&str, Vec<String>>(key).await {
            Ok(k) => k,
            Err(e) => {
                error!("Redis error while listing keys: {}", e);
                return Err(CacheError::OperationError(e.to_string()));
            }
        };
        let mut count: u32 = 0;
        for key in keys {
            let result = self.conn.hdel::<&str, &str, u32>(&key, field).await;

            match result {
                Ok(v) => count += v,
                Err(e) => {
                    error!("Redis error while deleting key \"{}\": {}", key, e);
                    return Err(CacheError::OperationError(e.to_string()));
                }
            }
        }
        Ok(Some(count))
    }

    async fn hpurge(&mut self) -> Result<Option<u32>, CacheError> {
        let result = self.conn.flushdb().await;

        match result {
            Ok(()) => {
                info!("Redis cache purged successfully.");
                Ok(None) // Redis doesn't return a count for FLUSHDB
            }
            Err(e) => {
                error!("Redis error while purging cache: {}", e);
                Err(CacheError::OperationError(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use pageshelf_core::cache::Cache;
    use testcontainers::{
        GenericImage,
        core::{IntoContainerPort, WaitFor},
        runners::AsyncRunner,
    };

    use crate::RedisCache;

    #[tokio::test]
    async fn redis_client_with_containerized_redis() {
        let container = GenericImage::new("redis", "latest")
            .with_exposed_port(6379.tcp())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .unwrap();
        let host = container.get_host().await.unwrap().to_string();
        let host_port = container.get_host_port_ipv4(6379).await.unwrap();

        let mut client = RedisCache::new(&host, host_port, None).unwrap();

        client.test().await.unwrap();
    }

    #[tokio::test]
    async fn redis_client_with_containerized_valkey() {
        let container = GenericImage::new("valkey/valkey", "latest")
            .with_exposed_port(6379.tcp())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .unwrap();
        let host = container.get_host().await.unwrap().to_string();
        let host_port = container.get_host_port_ipv4(6379).await.unwrap();

        let mut client = RedisCache::new(&host, host_port, None).unwrap();

        client.test().await.unwrap();
    }
}
