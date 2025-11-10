//! Upstream module
//!
//! Establishes interfaces that can be used to read page information and data.

mod location;
pub mod mock;
pub use location::{AssetLocation, PageLocation};
use std::{fmt::Display, path::Path, sync::Arc};
mod cached;
pub use cached::CachedUpstream;

/* -------------------------------------------------------------------------- */
/*                                   Errors                                   */
/* -------------------------------------------------------------------------- */

/// Represents an error that occurred during an operation involving an [`Upstream`].
#[derive(Debug, PartialEq, Eq)]
pub enum UpstreamError {
    /// An unexpected failure occurred within the [Upstream] provider.
    ///
    /// This is a general catch-all for errors that originate from the upstream system
    /// itself - such as network issues, authentication failures, etc.
    ///
    /// Consider logging or propagating the original cause if possible.
    ProviderError,
    /// The provided arguments for the [Upstream]'s operation were invalid or unsupported.
    ///
    /// This indicates that the caller supplied parameters that the upstream could not
    /// interpret or process. Examples include malformed identifiers, missing fields,
    /// or otherwise nonsensical input.
    InvalidArguments,
    /// The requested operation was not implemented for this [Upstream].
    ///
    /// Indicates that the upstream does not support the requested functionality.
    /// For optional features, implementations should handle this gracefully.
    NotImplemented,
    /// The requested resource were not available in the [Upstream] provider.
    ///
    /// Typically occurs when querying a non-existent owner, project, channel, or asset.
    NotFound,
}

impl Display for UpstreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProviderError => f.write_str("Provider error"),
            Self::InvalidArguments => f.write_str("Invalid arguments"),
            Self::NotImplemented => f.write_str("Not implemented"),
            Self::NotFound => f.write_str("Not found"),
        }
    }
}

impl std::error::Error for UpstreamError {}

/* -------------------------------------------------------------------------- */
/*                                   Traits                                   */
/* -------------------------------------------------------------------------- */

/* -------------------------------- Upstream -------------------------------- */

/// A read-only storage medium for projects, channels, and their assets.
///
/// The [`Upstream`] trait abstracts over any provider that can supply
/// project metadata and binary assets. Implementations may represent
/// remote APIs, on-disk caches, or mock test providers.
pub trait Upstream: Send + Sync {
    /// List all project owners available.
    ///
    /// # Errors
    ///
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    /// - `NotImplemented` - The upstream is not capable of performing this.
    fn list_owners(&self) -> impl Future<Output = Result<Arc<[String]>, UpstreamError>> + Send {
        async move { Err(UpstreamError::NotImplemented) }
    }

    /// List all projects available to a specific owner.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The owner wasn't available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    /// - `NotImplemented` - The upstream is not capable of performing this.
    #[allow(unused_variables)]
    fn list_projects(
        &self,
        owner: &str,
    ) -> impl Future<Output = Result<Arc<[String]>, UpstreamError>> + Send {
        async move { Err(UpstreamError::NotImplemented) }
    }

    /// List all channels available in a specific project.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The project wasn't available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    /// - `NotImplemented` - The upstream is not capable of performing this.
    #[allow(unused_variables)]
    fn list_channels(
        &self,
        owner: &str,
        project: &str,
    ) -> impl Future<Output = Result<Arc<[String]>, UpstreamError>> + Send {
        async move { Err(UpstreamError::NotImplemented) }
    }

    /// List all assets contained within a page.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The page wasn't available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    /// - `NotImplemented` - The upstream is not capable of performing this.
    #[allow(unused_variables)]
    fn list_assets(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> impl Future<Output = Result<Arc<[String]>, UpstreamError>> + Send {
        async move { Err(UpstreamError::NotImplemented) }
    }

    /// Returns all bytes contained in an asset from a page.
    ///
    /// The bytes are formatted as plain data, and will be sourced from the latest available version.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The page or asset was not available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    #[allow(unused_variables)]
    fn get_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
    ) -> impl Future<Output = Result<Arc<[u8]>, UpstreamError>> + Send;

    /// Returns all bytes contained from the first given asset that can be found in a page.
    ///
    /// The bytes are formatted as plain data, and will be sourced from the latest available version.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The page or asset was not available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    fn get_first_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        paths: &[&Path],
    ) -> impl Future<Output = Result<(usize, Arc<[u8]>), UpstreamError>> + Send {
        async move {
            let mut idx = 0;
            for path in paths {
                match self.get_asset_bytes(owner, project, channel, path).await {
                    Ok(v) => return Ok((idx, v)),
                    Err(UpstreamError::NotFound) => {
                        idx += 1;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }

            Err(UpstreamError::NotFound)
        }
    }

    /// Returns the current version or revision of a given page.
    ///
    /// This can be used to perform cache invalidation.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The page or asset was not available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    #[allow(unused_variables)]
    fn get_page_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> impl Future<Output = Result<String, UpstreamError>> + Send;

    /// Returns if a given page is available.
    ///
    /// # Errors
    ///
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    fn has_page(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> impl Future<Output = Result<bool, UpstreamError>> + Send {
        async move {
            match self.list_channels(owner, project).await {
                Ok(chans) => Ok(chans.iter().any(|c| c == channel)),
                Err(UpstreamError::NotImplemented) => Ok(false),
                Err(e) => Err(e),
            }
        }
    }
}
