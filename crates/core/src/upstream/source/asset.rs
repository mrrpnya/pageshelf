use std::{path::Path, sync::Arc};

use crate::upstream::UpstreamError;

/// Retrieves assets from the latest version of a page.
pub trait AssetSource: Send + Sync {
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

    /// Returns all bytes contained from the first given asset that can be found in the a page.
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
}

/// Can list assets from the latest version of a page.
pub trait AssetListSource: Send + Sync {
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
}

/// Retrieves assets from a specific version of a page.
pub trait VersionedAssetSource: Send + Sync {
    /// Returns all bytes contained in an asset from a specific version of a page.
    ///
    /// The bytes are formatted as plain data, and will be sourced from the latest available version.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The page or asset was not available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    #[allow(unused_variables)]
    fn get_version_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        version: &str,
        path: &Path,
    ) -> impl Future<Output = Result<Arc<[u8]>, UpstreamError>> + Send;

    /// Returns all bytes contained from the first given asset that can be found in a specific version of a page.
    ///
    /// The bytes are formatted as plain data, and will be sourced from the latest available version.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The page or asset was not available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    fn get_version_first_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        version: &str,
        paths: &[&Path],
    ) -> impl Future<Output = Result<(usize, Arc<[u8]>), UpstreamError>> + Send {
        async move {
            let mut idx = 0;
            for path in paths {
                match self
                    .get_version_asset_bytes(owner, project, channel, version, path)
                    .await
                {
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
}

/// Can list assets from the latest version of a page.
pub trait VersionedAssetListSource: Send + Sync {
    /// List all assets contained within a page.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The page wasn't available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    /// - `NotImplemented` - The upstream is not capable of performing this.
    #[allow(unused_variables)]
    fn list_version_assets(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        version: &str,
    ) -> impl Future<Output = Result<Arc<[String]>, UpstreamError>> + Send {
        async move { Err(UpstreamError::NotImplemented) }
    }
}
