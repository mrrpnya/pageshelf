use std::{path::Path, sync::Arc};

use color_eyre::eyre::{self, Context};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};

use crate::{
    cache::{Cache, CacheConnection, CacheError},
    ext::Normalizable,
};

// TODO: Improve doc comment style?

/// A key-value data cache; Can store arbitrary information within it.
///
/// It is intended as an abstraction over popular key-value caches like Redis or Valkey.
pub trait KVHashCache: Cache {
    /// The connection type for this cache, used to query and mutate it.
    type Connection<'a>: KVHashCacheConnection + Send
    where
        Self: 'a;
}

/// A connection to a key-value cache that can use hashes with fields (e.g. Redis, Valkey).
///
/// When implemented, this automatically implements CacheConnection.
pub trait KVHashCacheConnection {
    /// Sets the value of a hash field within the Cache.
    /// If it does not already exist, it will be created.
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) - The hash in the cache to put the field in
    /// - `field` (`&str`) - The field within the hash to assign
    /// - `value` (`&[u8]`) - The data to assign to the field
    ///
    /// # Errors
    ///
    /// - [`OperationError`](CacheError::OperationError) - Failed to apply the value due to an internal error.
    ///
    /// # Examples
    ///
    /// ```
    /// use pageshelf_core::cache::{Cache, MockCache, KVHashCacheConnection};
    /// use std::ops::Deref;
    ///
    /// async {
    ///     let cache = MockCache::default();
    ///     let mut conn = cache.connect().await.unwrap();
    ///
    ///     // Insert data
    ///     let data = b"The quick brown fox";
    ///     conn.hset("MyObject", "foo", data).await.unwrap();
    ///
    ///     // Data is now in the cache
    ///     let value = conn.hget("MyObject", "foo").await.unwrap();
    ///     assert_eq!(value.as_ref(), data);
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    fn hset(
        &mut self,
        key: &str,
        field: &str,
        value: &[u8],
    ) -> impl Future<Output = Result<(), CacheError>> + Send;

    /// Gets a value from a hash field stored in the Cache.
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) - The hash in the cache to find the field in
    /// - `field` (`&str`) - The field within the hash to grab the value from
    ///
    /// # Returns
    ///
    /// - `Arc<[u8]>` - The data stored in the cache
    ///
    /// # Errors
    ///
    /// - [`NotFound`](CacheError::NotFound) - Could not find the data within the cache.
    /// - [`OperationError`](CacheError::OperationError) - Failed to get the value due to an internal error.
    ///
    /// # Examples
    ///
    /// ```
    /// use pageshelf_core::cache::{Cache, CacheError, MockCache, KVHashCacheConnection};
    /// use std::ops::Deref;
    ///
    /// async {
    ///     let cache = MockCache::default();
    ///     let mut conn = cache.connect().await.unwrap();
    ///
    ///     // There's nothing to get
    ///     assert_eq!(conn.hget("MyObject", "foo").await, Err(CacheError::NotFound));
    ///
    ///     // Insert data
    ///     let data = b"The quick brown fox";
    ///     conn.hset("MyObject", "foo", data).await.unwrap();
    ///
    ///     // Now there is
    ///     let value = conn.hget("MyObject", "foo").await.unwrap();
    ///     assert_eq!(value.as_ref(), data);
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    fn hget(
        &self,
        key: &str,
        field: &str,
    ) -> impl Future<Output = Result<Arc<[u8]>, CacheError>> + Send;

    /// Gets the first available field within a cached hash
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) - The hash in the cache to search for the fields in
    /// - `field` (`&str`) - The fields to check for
    ///
    /// # Returns
    ///
    /// - `(usize, Arc<[u8]>)` - The index of the returned field and data stored within the field.
    ///
    /// # Errors
    ///
    /// - [`NotFound`](CacheError::NotFound) - Could not find the data within the cache.
    /// - [`OperationError`](CacheError::OperationError) - Failed to get the value due to an internal error.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use crate::...;
    ///
    /// async {
    ///   let fields = ["bbb", "aaa"];
    ///
    ///   let _ = cache.hset("MyObject", "aaa", "bar").await;
    ///
    ///   assert_eq!(cache.hget_first_field("MyObject", &fields).await.unwrap(), (1, "bar"))
    /// };
    /// ```
    fn hget_first_field(
        &self,
        key: &str,
        fields: &[&str],
    ) -> impl Future<Output = Result<(usize, Arc<[u8]>), CacheError>> {
        async move {
            // Default implementation (recommend overriding this in impls if you can accelerate it)
            let mut i = 0;
            for field in fields {
                match self.hget(key, field).await {
                    Ok(v) => return Ok((i, v)),
                    Err(CacheError::NotFound) => {
                        i += 1;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }

            Err(CacheError::NotFound)
        }
    }

    /// Returns a hash field as a string.
    ///
    /// Abstraction over cache.hget() that automatically handles UTF-8 string interpretation.
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) - The hash in the cache to find the field in
    /// - `field` (`&str`) - The field within the hash to grab the value from
    ///
    /// # Errors
    ///
    /// - `NotFound` - Could not find the data within the cache.
    /// - `OperationError` - Failed to apply the value due to an internal error.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use crate::...;
    ///
    /// async {
    ///   let _ = cache.hset("MyObject", "foo", "bar").await;
    ///     
    ///   assert_eq!(conn.hget_string("MyObject", "foo").await.unwrap(), "bar");
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    async fn hget_string(&self, key: &str, field: &str) -> Result<String, CacheError> {
        let result = self.hget(key, field).await;

        match result {
            Ok(v) => {
                let str = std::str::from_utf8(&v);
                match str {
                    Ok(v) => Ok(v.to_string()),
                    Err(e) => {
                        let msg = format!("Could not interpret hash field value as string: {e}");
                        tracing::error!(msg);
                        Err(CacheError::OperationError(msg))
                    }
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Deletes a hash, and all of its fields.
    fn hdel(&mut self, key: &str) -> impl Future<Output = Result<Option<u32>, CacheError>> + Send;
    /// Deletes a specific field from a hash.
    fn hdel_field(
        &mut self,
        key: &str,
        field: &str,
    ) -> impl Future<Output = Result<Option<u32>, CacheError>> + Send;

    /// Delete all contents of the cache.
    fn hpurge(&mut self) -> impl Future<Output = Result<Option<u32>, CacheError>> + Send;
}

impl<T> CacheConnection for T
where
    T: KVHashCacheConnection + Send + Sync,
{
    /* ---------------------------------- Write --------------------------------- */

    async fn set_asset(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
        asset: &[u8],
    ) -> Result<(), CacheError> {
        let hash = hash_owner_project_channel("asset", owner, project, channel);
        match path.normalized_absolute().to_str() {
            Some(path) => {
                let asset = compress_prepend_size(asset);
                self.hset(&hash, path, &asset).await.map_err(|e| {
                    let msg: String = format!("Failed to set hash field to asset data: {e}");
                    tracing::error!(msg);
                    CacheError::OperationError(msg)
                })
            }
            None => {
                let msg = format!("Error interpreting path {path:?}");
                tracing::error!(msg);
                Err(CacheError::OperationError(msg))
            }
        }
    }

    async fn set_page_version(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
        version: &[u8],
    ) -> Result<(), CacheError> {
        let hash = hash_owner_project_channel("page", owner, project, channel);
        self.hset(&hash, "version", version).await.map_err(|e| {
            let msg: String = format!("Failed to set hash field to asset data: {e}");
            tracing::error!(msg);
            CacheError::OperationError(msg)
        })
    }

    /* ---------------------------------- Read ---------------------------------- */

    async fn get_asset(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
    ) -> Result<Arc<[u8]>, CacheError> {
        let hash = hash_owner_project_channel("asset", owner, project, channel);
        match path.normalized_absolute().to_str() {
            Some(path) => match self.hget(&hash, path).await {
                Ok(data) => match decompress_size_prepended(&data) {
                    Ok(data) => Ok(Arc::from(data)),
                    Err(e) => {
                        let msg = format!("Error decompressing cached asset: {e}");
                        tracing::error!(msg);
                        Err(CacheError::OperationError(msg))
                    }
                },
                Err(e) => Err(e),
            },
            None => {
                let msg = format!("Error interpreting path {path:?}");
                tracing::error!(msg);
                Err(CacheError::OperationError(msg))
            }
        }
    }

    async fn get_page_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<Arc<[u8]>, CacheError> {
        let hash = hash_owner_project_channel("page", owner, project, channel);

        match self.hget(&hash, "version").await {
            Ok(data) => Ok(data),
            Err(e) => Err(e),
        }
    }

    /* --------------------------------- Delete --------------------------------- */

    async fn purge(&mut self) -> Result<Option<u32>, CacheError> {
        self.hpurge().await
    }

    async fn delete_page(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<(), CacheError> {
        let hash = hash_owner_project_channel("asset", owner, project, channel);

        self.hdel(&hash).await.map(|_| ())
    }

    async fn delete_asset(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
    ) -> Result<(), CacheError> {
        let hash = hash_owner_project_channel("asset", owner, project, channel);

        match path.normalized_absolute().to_str() {
            Some(path) => self.hdel_field(&hash, path).await.map(|_| ()),
            None => {
                let msg = format!("Error interpreting path {path:?}");
                tracing::error!(msg);
                Err(CacheError::OperationError(msg))
            }
        }
    }

    /* --------------------------------- Testing -------------------------------- */

    async fn test(&mut self) -> color_eyre::eyre::Result<String, eyre::Report> {
        use std::fmt::Write;

        let mut report = String::new();
        writeln!(report, "🔍 Starting cache self-test...")?;

        let key = "TestObject";
        let field1 = "foo";
        let field2 = "bar";
        let field3 = "baz";
        let value1 = b"value1";
        let value2 = b"value2";
        let value3 = b"value3";

        // --- Basic hset/hget ---
        self.hset(key, field1, value1)
            .await
            .wrap_err("Failed to set field1")?;
        self.hset(key, field2, value2)
            .await
            .wrap_err("Failed to set field2")?;
        writeln!(report, "✅ hset: set fields 'foo', 'bar'")?;

        let retrieved1 = self
            .hget(key, field1)
            .await
            .wrap_err("Failed to get field1")?;
        let retrieved2 = self
            .hget(key, field2)
            .await
            .wrap_err("Failed to get field2")?;
        if &*retrieved1 == value1 && &*retrieved2 == value2 {
            writeln!(report, "✅ hget: retrieved correct values")?;
        } else {
            eyre::bail!("❌ hget mismatch");
        }

        // --- hget_first_field ---
        let (index, data) = self
            .hget_first_field(key, &[field2, field1])
            .await
            .wrap_err("Failed hget_first_field (set 1)")?;
        if index == 0 && &*data == value2 {
            writeln!(report, "✅ hget_first_field (set 1): correct")?;
        } else {
            eyre::bail!("❌ hget_first_field returned wrong field (1)");
        }

        let (index, data) = self
            .hget_first_field(key, &[field3, field1])
            .await
            .wrap_err("Failed hget_first_field (set 2)")?;
        if index == 1 && &*data == value1 {
            writeln!(report, "✅ hget_first_field (set 2): correct")?;
        } else {
            eyre::bail!("❌ hget_first_field returned wrong field (2)");
        }

        // --- hget_string ---
        let string_value = self
            .hget_string(key, field1)
            .await
            .wrap_err("Failed hget_string")?;
        if string_value == "value1" {
            writeln!(report, "✅ hget_string: correct value")?;
        } else {
            eyre::bail!("❌ hget_string mismatch");
        }

        // --- Overwriting values ---
        self.hset(key, field1, value3)
            .await
            .wrap_err("Failed to overwrite field1")?;
        let retrieved = self
            .hget(key, field1)
            .await
            .wrap_err("Failed to get overwritten field1")?;
        if &*retrieved == value3 {
            writeln!(report, "✅ Overwrite field: success")?;
        } else {
            eyre::bail!("❌ Overwrite failed");
        }

        // --- Deleting keys ---
        let _ = self
            .hdel("NonExistent")
            .await
            .wrap_err("Failed to delete non-existent key")?;
        writeln!(report, "✅ delete: non-existent key handled cleanly")?;

        let _ = self.hdel(key).await.wrap_err("Failed to delete test key")?;

        match self.hget(key, field1).await {
            Err(CacheError::NotFound) => writeln!(report, "✅ Verified key deletion")?,
            Ok(_) => eyre::bail!("❌ Key still exists after delete"),
            Err(e) => eyre::bail!("❌ Unexpected error after delete: {:?}", e),
        }

        writeln!(report, "🎉 Cache self-test completed successfully!")?;
        Ok(report)
    }

    async fn get_first_asset(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &[&Path],
    ) -> Result<(usize, Arc<[u8]>), CacheError> {
        let mut i = 0;
        for p in path {
            match self.get_asset(owner, project, channel, p).await {
                Ok(asset) => return Ok((i, asset)),
                Err(CacheError::NotFound) => {
                    i += 1;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(CacheError::NotFound)
    }
}

/* -------------------------------- Utilities ------------------------------- */

fn hash_owner_project_channel(
    namespace: &str,
    owner: &str,
    project: &str,
    channel: &str,
) -> String {
    blake3::Hasher::new()
        .update(namespace.as_bytes())
        .update(owner.as_bytes())
        .update(project.as_bytes())
        .update(channel.as_bytes())
        .finalize()
        .to_string()
}
