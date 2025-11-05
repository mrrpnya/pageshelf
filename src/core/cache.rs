//! Simple cache abstractions.
//!
//! Provides easily-implementable abstractions for implementing key-value caches.
//! Such implementations can then be used to improve performance.

// TODO: Consider making this focus more on operating with pages at a high level as opposed to expression-based data operations?

use std::fmt::Display;

#[derive(Debug, PartialEq, Eq)]
pub enum CacheError {
    /// A problem occurred when trying to connect to the cache.
    ConnectionError,
    /// A problem occurred when trying to do something with the cache.
    OperationError(String),
    /// The desired item was not found in the cache.
    NotFound,
}

impl Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ConnectionError => write!(f, "Connection Error"),
            Self::OperationError(s) => write!(f, "Operation Error: {s}"),
            Self::NotFound => write!(f, "Not Found"),
        }
    }
}

impl std::error::Error for CacheError {}

/// A data cache. It can store arbitrary information within it.
///
/// It is intended as an abstraction over popular key-value caches like Redis or Valkey;
/// As such, it is expected to be a key-value store, with Regex support.
pub trait Cache: Clone + Send + Sync {
    /// The connection type for this cache, used to query and mutate it.
    type Connection<'a>: CacheConnection + Send
    where
        Self: 'a;

    /// Connects to the cache, which allows querying and mutating it.
    ///
    /// # Returns
    ///
    /// - `Result<Self::Connection, CacheError>` - The connection if successful, otherwise an error.
    ///
    /// # Errors
    ///
    /// Describe possible errors.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use crate::...;
    ///
    /// async {
    ///   let conn = cache.connect().await.unwrap();
    ///
    ///   // With a connection, you can do stuff with the Cache
    ///   let _ = conn.hset("MyObject", "foo", "bar").await;
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    async fn connect<'a>(&'a self) -> Result<Self::Connection<'a>, CacheError>;
}

/// An active connection to a cache. This allows you to query or mutate the cache.
///
/// To leverage certain caches effectively, explicit, scoped connections are used.
pub trait CacheConnection: Send {
    /// Sets the value of a hash field within the Cache.
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) - The hash in the cache to put the field in
    /// - `field` (`&str`) - The field within the hash to assign
    /// - `value` (`&[u8]`) - The data to assign to the field
    ///
    /// # Returns
    ///
    /// - `Result<(), CacheError>` - Nothing on successful assignment.
    ///   If an error occurred, CacheError will be returned instead.
    ///
    /// # Errors
    ///
    /// - `OperationError` - Failed to apply the value due to an internal error.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use crate::...;
    ///
    /// async {
    ///   assert!(cache.hget("MyObject", "foo").await.is_err())
    ///   let _ = cache.hset("MyObject", "foo", data).await;
    ///   assert_eq!(cache.hget("MyObject", "foo").await.unwrap(), data)
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    async fn hset(&mut self, key: &str, field: &str, value: &[u8]) -> Result<(), CacheError>;

    /// Gets a value from a hash field stored in the Cache.
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) - The hash in the cache to find the field in
    /// - `field` (`&str`) - The field within the hash to grab the value from
    ///
    /// # Returns
    ///
    /// - `Result<Vec<u8>, CacheError>` - The data stored in the cache, otherwise an error.
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
    ///   // You should now be able to get "VALUE_1" from the cache
    ///   assert_eq!(cache.hget("MyObject", "foo").await.unwrap(), "bar")
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    async fn hget(&mut self, key: &str, field: &str) -> Result<Vec<u8>, CacheError>;

    /// Gets the first available field within a cached hash
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) - The hash in the cache to search for the fields in
    /// - `field` (`&str`) - The fields to check for
    ///
    /// # Returns
    ///
    /// - `Result<(usize, Vec<u8>), CacheError>` - The field index and data stored within the field, otherwise an error.
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
    ///   let fields = ["bbb", "aaa"];
    ///
    ///   let _ = cache.hset("MyObject", "aaa", "bar").await;
    ///
    ///   assert_eq!(cache.hget_first_field("MyObject", &fields).await.unwrap(), (1, "bar"))
    /// };
    /// ```
    async fn hget_first_field(
        &mut self,
        key: &str,
        fields: &[&str],
    ) -> Result<(usize, Vec<u8>), CacheError> {
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

    /// Abstraction over cache.hget() that automatically handles UTF-8 string interpretation
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) - The hash in the cache to find the field in
    /// - `field` (`&str`) - The field within the hash to grab the value from
    ///
    /// # Returns
    ///
    /// - `Result<String, CacheError>` - The string stored in the cache, otherwise an error.
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
    async fn hget_string(&mut self, key: &str, field: &str) -> Result<String, CacheError> {
        let result = self.hget(key, field).await;

        match result {
            Ok(v) => {
                let str = std::str::from_utf8(&v);
                match str {
                    Ok(v) => Ok(v.to_string()),
                    Err(e) => Err(CacheError::OperationError(format!("UTF-8 Error: {}", e))),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Removes an entire hash from the Cache's storage and all of its fields
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) -  The hash in the cache to delete.
    ///
    /// # Returns
    ///
    /// - `Result<u32, CacheError>` - The amount of keys deleted from the cache if successful, otherwise an error.
    ///
    /// # Errors
    ///
    /// - `OperationError` - Failed to apply the value due to an internal error.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use crate::...;
    ///
    /// async {
    ///   let _ = cache.hset("MyObject", "foo", "bar").await;
    ///   assert_eq!(cache.hget("MyObject", "foo").await.unwrap(), "VALUE_1")
    ///
    ///   let _ = cache.delete("MyObject").await;
    ///   
    ///   // "MyObject" should no longer be available in the cache
    ///   assert_eq!(cache.hget("MyObject", "foo").await.unwrap(), "VALUE_1")
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    async fn delete(&mut self, key: &str) -> Result<u32, CacheError>;

    /// Deletes everything from the cache
    ///
    /// # Errors
    ///
    /// - `OperationError` - Failed due to an internal error.
    async fn purge(&mut self) -> Result<Option<u32>, CacheError>;
}
use color_eyre::eyre::{self, Result, WrapErr};

/// A helper function to test a type is implementing `Cache` correctly
///
/// # Arguments
///
/// - `cache` - Any instance of a type that implements `Cache`
///
/// # Panics
///
/// Panics if any of the cache operations fail unexpectedly.
pub async fn test_cache<C: Cache>(cache: C) -> Result<String> {
    // Connect to the cache
    let mut conn = cache
        .connect()
        .await
        .wrap_err("Failed to connect to cache")?;

    test_cache_conn(&mut conn).await
}

/// A helper function to test a type is implementing `CacheConnection` correctly
///
/// # Arguments
///
/// - `conn` - Any instance of a type that implements `CacheConnection`
///
/// # Panics
///
/// Panics if any of the cache operations fail unexpectedly.
pub async fn test_cache_conn<C: CacheConnection>(conn: &mut C) -> Result<String> {
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
    conn.hset(key, field1, value1)
        .await
        .wrap_err("Failed to set field1")?;
    conn.hset(key, field2, value2)
        .await
        .wrap_err("Failed to set field2")?;
    writeln!(report, "✅ hset: set fields 'foo', 'bar'")?;

    let retrieved1 = conn
        .hget(key, field1)
        .await
        .wrap_err("Failed to get field1")?;
    let retrieved2 = conn
        .hget(key, field2)
        .await
        .wrap_err("Failed to get field2")?;
    if retrieved1 == value1 && retrieved2 == value2 {
        writeln!(report, "✅ hget: retrieved correct values")?;
    } else {
        eyre::bail!("❌ hget mismatch");
    }

    // --- hget_first_field ---
    let (index, data) = conn
        .hget_first_field(key, &[field2, field1])
        .await
        .wrap_err("Failed hget_first_field (set 1)")?;
    if index == 0 && data == value2 {
        writeln!(report, "✅ hget_first_field (set 1): correct")?;
    } else {
        eyre::bail!("❌ hget_first_field returned wrong field (1)");
    }

    let (index, data) = conn
        .hget_first_field(key, &[field3, field1])
        .await
        .wrap_err("Failed hget_first_field (set 2)")?;
    if index == 1 && data == value1 {
        writeln!(report, "✅ hget_first_field (set 2): correct")?;
    } else {
        eyre::bail!("❌ hget_first_field returned wrong field (2)");
    }

    // --- hget_string ---
    let string_value = conn
        .hget_string(key, field1)
        .await
        .wrap_err("Failed hget_string")?;
    if string_value == "value1" {
        writeln!(report, "✅ hget_string: correct value")?;
    } else {
        eyre::bail!("❌ hget_string mismatch");
    }

    // --- Overwriting values ---
    conn.hset(key, field1, value3)
        .await
        .wrap_err("Failed to overwrite field1")?;
    let retrieved = conn
        .hget(key, field1)
        .await
        .wrap_err("Failed to get overwritten field1")?;
    if retrieved == value3 {
        writeln!(report, "✅ Overwrite field: success")?;
    } else {
        eyre::bail!("❌ Overwrite failed");
    }

    // --- Deleting keys ---
    let _ = conn
        .delete("NonExistent")
        .await
        .wrap_err("Failed to delete non-existent key")?;
    writeln!(report, "✅ delete: non-existent key handled cleanly")?;

    let deleted = conn
        .delete(key)
        .await
        .wrap_err("Failed to delete test key")?;
    if deleted > 0 {
        writeln!(report, "✅ delete: removed test key")?;
    } else {
        eyre::bail!("❌ delete failed to remove key");
    }

    match conn.hget(key, field1).await {
        Err(CacheError::NotFound) => writeln!(report, "✅ Verified key deletion")?,
        Ok(_) => eyre::bail!("❌ Key still exists after delete"),
        Err(e) => eyre::bail!("❌ Unexpected error after delete: {:?}", e),
    }

    writeln!(report, "🎉 Cache self-test completed successfully!")?;
    Ok(report)
}
