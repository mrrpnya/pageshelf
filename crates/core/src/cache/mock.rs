use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::cache::{Cache, KVHashCacheConnection};

/* -------------------------------------------------------------------------- */
/*                                    Cache                                   */
/* -------------------------------------------------------------------------- */

// The data a hash field can contain.
type HashField = Arc<[u8]>;

// Contains all the data a hash can keeps.
type DataHash = HashMap<String, HashField>;

/// Thread-safe hash storage.
type DataMap = Arc<RwLock<HashMap<String, DataHash>>>;

/// A fake [Cache] meant to assist in testing.
#[derive(Clone, Default, Debug)]
pub struct MockCache {
    asset_map: DataMap,
}

impl Cache for MockCache {
    type Connection<'a>
        = MockCacheConnection<'a>
    where
        Self: 'a;

    async fn connect<'a>(&'a self) -> Result<Self::Connection<'a>, super::CacheError> {
        Ok(MockCacheConnection { cache: self })
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Connection                                 */
/* -------------------------------------------------------------------------- */

/// Connection to a fake cache that is able to query and modify it.
pub struct MockCacheConnection<'a> {
    cache: &'a MockCache,
}

impl<'a> KVHashCacheConnection for MockCacheConnection<'a> {
    /* --------------------------------- Setting -------------------------------- */
    async fn hget(&self, key: &str, field: &str) -> Result<Arc<[u8]>, super::CacheError> {
        let r = self.cache.asset_map.read().await;
        if let Some(h) = r.get(key) {
            match h.get(field) {
                Some(data) => {
                    return Ok(data.clone());
                }
                None => {
                    return Err(super::CacheError::NotFound);
                }
            }
        }
        Err(super::CacheError::NotFound)
    }

    /* --------------------------------- Getting -------------------------------- */

    async fn hset(
        &mut self,
        key: &str,
        field: &str,
        value: &[u8],
    ) -> Result<(), super::CacheError> {
        let mut w = self.cache.asset_map.write().await;
        let hash = match w.get_mut(key) {
            Some(h) => h,
            None => {
                w.insert(key.to_owned(), HashMap::new());
                w.get_mut(key).expect(
                    "A value that was just put in the mock cache data was not found somehow?",
                )
            }
        };

        hash.insert(field.to_owned(), Arc::from(value));

        Ok(())
    }

    /* -------------------------------- Deletion -------------------------------- */

    async fn hdel(&mut self, key: &str) -> Result<Option<u32>, super::CacheError> {
        let mut w = self.cache.asset_map.write().await;
        match w.remove(key) {
            Some(_) => Ok(Some(1)),
            None => Ok(Some(0)),
        }
    }

    async fn hdel_field(
        &mut self,
        key: &str,
        field: &str,
    ) -> Result<Option<u32>, super::CacheError> {
        let mut w = self.cache.asset_map.write().await;
        if let Some(h) = w.get_mut(key) {
            match h.remove(field) {
                Some(_) => {
                    return Ok(Some(1));
                }
                None => {
                    return Ok(Some(0));
                }
            }
        }
        Ok(Some(0))
    }

    async fn hpurge(&mut self) -> Result<Option<u32>, super::CacheError> {
        // TODO: Count support?
        let mut w = self.cache.asset_map.write().await;
        w.clear();
        Ok(None)
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use crate::cache::{Cache, MockCache};

    #[tokio::test]
    async fn test_mock_cache() {
        let mut cache = MockCache::default();
        cache.test().await.unwrap();
    }
}
