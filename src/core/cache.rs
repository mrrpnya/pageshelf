//! Simple cache abstractions.
//!
//! Provides easily-implementable abstractions for implementing key-value caches.
//! Such implementations can then be used to improve performance.

/// Caches allow for fast, local storage of page data.
/// To leverage certain caches effectively, explicit, scoped connections are used.
#[derive(Debug, PartialEq, Eq)]
pub enum CacheError {
    /// A problem occurred when trying to connect to the cache.
    ConnectionError,
    /// A problem occurred when trying to do something with the cache.
    OperationError(String),
    /// The desired item was not found in the cache.
    NotFound,
}

/// A cache. It can store information within it.
///
/// It is intended as an abstraction over popular caches like Redis or Valkey;
/// As such, it is expected to be a key-value store, with Regex support.
pub trait Cache: Clone {
    type Connection: CacheConnection;
    /// Describe this function.
    ///
    /// # Returns
    ///
    /// - `Result<Self::Connection, CacheError>` - Describe the return value.
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
    ///   // Now that we have a connection, you can do stuff with the Cache
    ///   let _ = conn.set("KEY", "VALUE").await;
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    async fn connect(&self) -> Result<Self::Connection, CacheError>;
}

/// An active connection to a cache. This allows you to query or modify the cache.
pub trait CacheConnection {
    /// Sets a value in the Cache's stored data
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) - The location in the cache to apply the value to
    /// - `value` (`&[u8]`) - The data to assign to this key
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
    ///   assert!(cache.get("KEY_1").await.is_err())
    ///   let _ = cache.set("KEY_1", data).await;
    ///   assert_eq!(cache.get("KEY_1").await.unwrap(), data)
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    async fn set(&mut self, key: &str, value: &[u8]) -> Result<(), CacheError>;

    /// Gets a value from the Cache's stored data.
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) - The location in the cache to find the value in
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
    ///   let _ = cache.set("KEY_1", "VALUE_1").await;
    ///
    ///   // You should now be able to get "VALUE_1" from the cache
    ///   assert_eq!(cache.get("KEY_1").await.unwrap(), "VALUE_1")
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    async fn get(&mut self, key: &str) -> Result<Vec<u8>, CacheError>;

    /// Abstraction over cache.get() that automatically handles UTF-8 string interpretation
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) - The location in the cache to find the value in
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
    ///   let result = conn.get_string("KEY").await;
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    async fn get_string(&mut self, key: &str) -> Result<String, CacheError> {
        let result = self.get(key).await;

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

    /// Removes data from the Cache's storage.
    ///
    /// # Arguments
    ///
    /// - `key` (`&str`) -  The location(s) in the cache to delete.
    ///   Use Regex to help delete multiple.
    ///
    /// # Returns
    ///
    /// - `Result<u32, CacheError>` - The amount of keys deleted from the cache if successful, otherwise an error.
    ///
    /// # Errors
    ///
    /// - `NotFound` - Could not find the data to delete
    /// - `OperationError` - Failed to apply the value due to an internal error.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use crate::...;
    ///
    /// async {
    ///   let _ = cache.set("KEY_1", "VALUE_1").await;
    ///   assert_eq!(cache.get("KEY_1").await.unwrap(), "VALUE_1")
    ///
    ///   let _ = cache.delete("KEY_1").await;
    ///   
    ///   // "KEY_1" should no longer be available in the cache
    ///   assert!(cache.get("KEY_1").await.is_err());
    ///
    ///   // This should delete all other keys beginning with "KEY_"
    ///   let _ = cache.delete("KEY_*").await;
    /// };
    /// ```
    #[allow(async_fn_in_trait)]
    async fn delete(&mut self, key: &str) -> Result<u32, CacheError>;
}
