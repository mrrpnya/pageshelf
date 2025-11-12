//! Upstream module
//!
//! Establishes interfaces that can be used to read page information and data.

mod location;
pub mod mock;
pub use location::{AssetLocation, PageLocation};
use std::{fmt::Display, path::Path, sync::Arc};
pub mod managers;

use crate::upstream::source::{
    AssetListSource, AssetSource, PageListComponentsSource, PageListSource, PageVersionSource,
};
pub mod source;

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
    // TODO: Consider removing this? Stuff is already moved to smaller traits...
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
pub trait Upstream: Send + Sync + AssetSource + PageVersionSource {}
