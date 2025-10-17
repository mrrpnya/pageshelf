//! A Cache that allows using Redis to cache page info and Assets.
//!
//! Redis is a high-speed in-memory cache with data durability.
//! It is useful for reducing queries upstream (especially when deployed off-site).
//! See <https://redis.io/> for more information about Redis itself.
use std::sync::Arc;

use log::error;
use redis::{AsyncCommands, Client, RedisError, aio::MultiplexedConnection};

use crate::{Cache, CacheConnection, CacheError};

#[derive(Clone)]
pub struct RedisCache {
    client: Arc<Client>,
    ttl: Option<u32>,
}

impl RedisCache {
    pub fn new(host: &str, port: u16, ttl: Option<u32>) -> Result<Self, RedisError> {
        let address = format!("redis://{}:{}", host, port);
        match redis::Client::open(address) {
            Ok(v) => Ok(Self {
                client: Arc::new(v),
                ttl,
            }),
            Err(e) => {
                error!("Failed to create Redis cache: {}", e);
                Err(e)
            }
        }
    }
}

impl Cache for RedisCache {
    type Connection = RedisCacheConnection;
    async fn connect(&self) -> Result<Self::Connection, crate::CacheError> {
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

pub struct RedisCacheConnection {
    conn: MultiplexedConnection,
    ttl: Option<u32>,
}

impl CacheConnection for RedisCacheConnection {
    async fn set(&mut self, key: &str, value: &[u8]) -> Result<(), CacheError> {
        let result = self.conn.set(key, value).await;

        match result {
            Ok(()) => {}
            Err(e) => {
                error!("Redis error while setting key \"{}\"'s value: {}", key, e);
                return Err(CacheError::OperationError(e.to_string()));
            }
        }

        if self.ttl.is_some() {
            let ttl = self.ttl.unwrap();

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

    async fn get(&mut self, key: &str) -> Result<Vec<u8>, CacheError> {
        let exists = self.conn.exists::<&str, bool>(key).await;

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

        let result = self.conn.get::<&str, Vec<u8>>(key).await;

        match result {
            Ok(v) => Ok(v),
            Err(e) => {
                error!("Redis error while getting key \"{}\": {}", key, e);
                Err(CacheError::OperationError(e.to_string()))
            }
        }
    }

    async fn delete(&mut self, key: &str) -> Result<u32, CacheError> {
        let result = self.conn.del::<&str, u32>(key).await;

        match result {
            Ok(v) => Ok(v),
            Err(e) => {
                error!("Redis error while deleting key \"{}\": {}", key, e);
                Err(CacheError::OperationError(e.to_string()))
            }
        }
    }
}
