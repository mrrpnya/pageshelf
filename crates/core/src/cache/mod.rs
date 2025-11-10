//! Cache abstraction module
//!
//! Provides an interface for interacting with cache systems.
//! These interfaces can help with boosting performance elsewhere.

use color_eyre::eyre::{self, WrapErr};
use std::{fmt::Display, path::Path, sync::Arc};

mod kv_hash;
pub use kv_hash::*;
mod mock;
pub use mock::*;

/// Error type that is used to denote problems involved with caching.
#[derive(Debug, PartialEq, Eq)]
pub enum CacheError {
    /// A problem occurred when trying to connect to the cache.
    /// (e.g. network issues, service down, bad credentials)
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
            Self::OperationError(e) => write!(f, "Operation Error: {e}"),
            Self::NotFound => write!(f, "Not Found"),
        }
    }
}

impl std::error::Error for CacheError {}

/// Provides a way to connect to a cache.
#[allow(async_fn_in_trait)]
pub trait Cache: Clone + Send + Sync {
    /// The connection type for this cache, used to query and mutate it.
    type Connection<'a>: CacheConnection + Send
    where
        Self: 'a;

    /// Connects to the cache.
    ///
    /// Returns a connection to the cache, which allows querying and mutating it.
    ///
    /// # Errors
    ///
    /// - `CacheError::ConnectionError` - Failed to establish a connection.
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
    fn connect<'a>(
        &'a self,
    ) -> impl Future<Output = Result<Self::Connection<'a>, CacheError>> + Send;

    /// Performs a test on this cache to ensure it is working correctly.
    async fn test(&mut self) -> Result<String, eyre::Report> {
        test_cache(self).await
    }
}

/// An active connection to a cache. This allows you to query or mutate the cache.
///
/// To leverage certain caches effectively, explicit, scoped connections are used.
#[allow(async_fn_in_trait)]
pub trait CacheConnection: Send + Sync {
    /* -------------------------------- Utilities ------------------------------- */

    /// Performs a test on this connection to ensure it is working correctly
    async fn test(&mut self) -> Result<String, eyre::Report>;

    /* ------------------------------ Create/Update ----------------------------- */

    /// Saves an asset to the cache.
    ///
    /// # Errors
    ///
    /// - `ConnectionError` - There was a problem communicating with the cache.
    fn set_asset(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
        asset: &[u8],
    ) -> impl Future<Output = Result<(), CacheError>> + Send;

    /// Sets the version associated with a page.
    ///
    /// This can be used to determine if the data stored in the cache is out of date.
    ///
    /// # Errors
    ///
    /// - `ConnectionError` - There was a problem communicating with the cache.
    fn set_page_version(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
        version: &[u8],
    ) -> impl Future<Output = Result<(), CacheError>> + Send;

    /* ---------------------------------- Read ---------------------------------- */

    /// Fetches a specific asset within a page.
    ///
    /// # Errors
    ///
    /// - `ConnectionError` - There was a problem communicating with the cache.
    /// - `NotFound` - Could not find the asset to delete.
    fn get_asset(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
    ) -> impl Future<Output = Result<Arc<[u8]>, CacheError>> + Send;

    /// Finds and returns the first asset available in a list of assets.
    ///
    /// Returns the index within said list that was returned.
    ///
    /// # Errors
    ///
    /// - `ConnectionError` - There was a problem communicating with the cache.
    /// - `NotFound` - Could not find the asset to delete.
    fn get_first_asset(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &[&Path],
    ) -> impl Future<Output = Result<(usize, Arc<[u8]>), CacheError>> + Send {
        async move {
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

    /// Gets the version associated with a given page.
    ///
    /// This can be used to determine if the data stored in the cache is out of date.
    ///
    /// # Errors
    ///
    /// - `ConnectionError` - There was a problem communicating with the cache.
    /// - `NotFound` - Could not find the asset to delete.
    fn get_page_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> impl Future<Output = Result<Arc<[u8]>, CacheError>> + Send;

    /* --------------------------------- Delete --------------------------------- */

    /// Deletes a page from the cache and all assets and metadata associated with it.
    ///
    /// # Errors
    ///
    /// - `ConnectionError` - There was a problem communicating with the cache.
    /// - `NotFound` - Could not find the asset to delete.
    fn delete_page(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> impl Future<Output = Result<(), CacheError>> + Send;

    /// Deletes a specific asset from a page in the cache
    ///
    /// # Errors
    ///
    /// - `ConnectionError` - There was a problem communicating with the cache.
    /// - `NotFound` - Could not find the asset to delete.
    fn delete_asset(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
    ) -> impl Future<Output = Result<(), CacheError>> + Send;

    /// Deletes everything from the cache.
    ///
    /// Returns the amount of things deleted, if it counted it.
    ///
    /// # Errors
    ///
    /// - `ConnectionError` - There was a problem communicating with the cache.
    fn purge(&mut self) -> impl Future<Output = Result<Option<u32>, CacheError>> + Send;
}

/* -------------------------------------------------------------------------- */
/*                                   Testing                                  */
/* -------------------------------------------------------------------------- */

/// Tests a cache to ensure it works correctly.
///
/// This action can be destructive to the data contained within a cache.
async fn test_cache<C: Cache>(cache: &mut C) -> Result<String, eyre::Report> {
    let mut report = String::new();
    report.push_str("🔍 Starting cache self-test...\n");

    // Connect
    let mut conn = cache
        .connect()
        .await
        .wrap_err("❌ Failed to connect to cache")?;
    report.push_str("✅ Connected to cache successfully.\n");

    // Run connection internal test
    match conn.test().await {
        Ok(msg) => report.push_str(&format!("✅ Connection test passed: {msg}\n")),
        Err(err) => return Err(err.wrap_err("❌ Cache connection self-test failed")),
    }

    // Clean slate
    conn.purge()
        .await
        .map_err(|e| eyre::eyre!("❌ Failed to purge cache: {e}"))?;
    report.push_str("✅ Cache purged.\n");

    // Sample assets
    let asset1 = b"foo";
    let asset2 = b"bar";
    let owner = "test_owner";
    let project = "test_project";
    let channel = "test_channel";
    let path1 = Path::new("path1");
    let path2 = Path::new("path2");
    let paths = &[path1, path2];

    // Test write & read
    conn.set_asset(owner, project, channel, path1, asset1)
        .await
        .map_err(|e| eyre::eyre!("❌ Failed to set asset1: {e}"))?;
    report.push_str("✅ Asset1 stored successfully.\n");

    let retrieved1 = conn
        .get_asset(owner, project, channel, path1)
        .await
        .map_err(|e| eyre::eyre!("❌ Failed to get asset1: {e}"))?;
    assert_eq!(&*retrieved1, asset1.as_slice(), "❌ Asset1 data mismatch");
    report.push_str("✅ Asset1 retrieved successfully.\n");

    // Ensure unrelated assets don't exist
    match conn.get_asset(owner, project, channel, path2).await {
        Err(CacheError::NotFound) => {
            report.push_str("✅ Unrelated asset path2 correctly missing.\n")
        }
        Ok(_) => return Err(eyre::eyre!("❌ Unexpected asset found at path2")),
        Err(e) => return Err(eyre::eyre!("❌ Unexpected error when checking path2: {e}")),
    }

    // Test get_first_asset
    conn.set_asset(owner, project, channel, path2, asset2)
        .await
        .map_err(|e| eyre::eyre!("❌ Failed to set asset2: {e}"))?;
    let (index, asset) = conn.get_first_asset(owner, project, channel, paths).await?;
    assert_eq!(index, 0, "❌ get_first_asset returned wrong index");
    assert_eq!(
        &*asset,
        asset1.as_slice(),
        "❌ get_first_asset returned wrong data"
    );
    report.push_str("✅ get_first_asset works correctly.\n");

    // Test page version
    let version = b"v1.0";
    conn.set_page_version(owner, project, channel, version)
        .await
        .map_err(|e| eyre::eyre!("❌ Failed to set page version: {e}"))?;
    let retrieved_version = conn
        .get_page_version(owner, project, channel)
        .await
        .map_err(|e| eyre::eyre!("❌ Failed to get page version: {e}"))?;
    assert_eq!(&*retrieved_version, version, "❌ Page version mismatch");
    report.push_str("✅ Page version stored and retrieved correctly.\n");

    // Test delete_asset
    conn.delete_asset(owner, project, channel, path1)
        .await
        .map_err(|e| eyre::eyre!("❌ Failed to delete asset1: {e}"))?;
    match conn.get_asset(owner, project, channel, path1).await {
        Err(CacheError::NotFound) => report.push_str("✅ Asset1 confirmed deleted.\n"),
        Ok(_) => return Err(eyre::eyre!("❌ Asset1 still exists after deletion!")),
        Err(e) => {
            return Err(eyre::eyre!(
                "❌ Unexpected error after deleting asset1: {e}"
            ));
        }
    }

    // Test delete_page (should remove asset2 too)
    conn.delete_page(owner, project, channel)
        .await
        .map_err(|e| eyre::eyre!("❌ Failed to delete page: {e}"))?;
    match conn.get_asset(owner, project, channel, path2).await {
        Err(CacheError::NotFound) => report.push_str("✅ Page deletion removed all assets.\n"),
        Ok(_) => return Err(eyre::eyre!("❌ Asset2 still exists after page deletion!")),
        Err(e) => return Err(eyre::eyre!("❌ Unexpected error after deleting page: {e}")),
    }

    // Final purge
    conn.purge()
        .await
        .map_err(|e| eyre::eyre!("❌ Final purge failed: {e}"))?;
    report.push_str("✅ Final purge successful.\n");

    report.push_str("🎉 Cache self-test complete.\n");
    Ok(report)
}
