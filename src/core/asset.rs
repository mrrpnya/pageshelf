use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum AssetError {
    NotFound,
    /// Corrupted data was found or was attempted to be processed.
    Corrupted,
    /// An error occurred within a provider.
    ProviderError,
}
/// Represents a file in a page.
pub trait Asset {
    /// Attempts to get the MIME type of this asset.
    ///
    /// # Returns
    ///
    /// - `Option<&str>` - The MIME type, if it was determined, otherwise None.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use crate::...;
    ///
    /// // Assume png_asset contains PNG data already
    /// let mime = png_asset.mime_type();
    /// assert_eq!(mime, "image/png")
    /// ```
    fn mime_type(&self) -> Option<&str> {
        None
    }
    fn bytes(&self) -> &[u8];
    fn into_bytes(self) -> Vec<u8>;

    fn body(&self) -> Result<&str, ()> {
        match std::str::from_utf8(self.bytes()) {
            Ok(v) => Ok(v),
            Err(_) => Err(()),
        }
    }
}

/// A trait that allows finding assets.
pub trait AssetSource {
    #[allow(async_fn_in_trait)]
    async fn get_asset(&self, path: &Path) -> Result<impl Asset, AssetError>;
    /// Returns the total number of bytes taken by all assets in this source.
    ///
    /// # Returns
    ///
    /// - `Option<u32>` - The amount of bytes taken if this source supports
    ///   counting this, otherwise None.
    fn total_bytes(&self) -> Option<u32> {
        None
    }
}

/// A trait that enables manipulation of assets on its implementors.
pub trait AssetWritable {
    fn set_asset(&mut self, path: &Path, asset: &impl Asset) -> Result<(), AssetError>;
    fn delete_asset(&mut self, path: &Path) -> Result<(), AssetError>;
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */
