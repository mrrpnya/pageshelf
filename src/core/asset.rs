use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum AssetError {
    /// The desired asset could not be found.
    NotFound,
    /// Corrupted data was found or was attempted to be processed.
    Corrupted,
    /// An error occurred within a provider.
    ProviderError,
    /// Unable to interpret the data of an asset in the desired manner
    CannotInterpret,
}

/// Represents a file that can be found in a page.
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

    /// The bytes that an asset contains - File content.
    ///
    /// # Returns
    ///
    /// - `&[u8]` - A reference to file data, stored in memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::pageshelf::Asset;
    /// use pageshelf::provider::memory::MemoryAsset;
    ///
    /// let control_data: Vec<u8> = vec![1, 2, 3];
    /// let asset = MemoryAsset::from(control_data.clone());
    /// let stored_data: &[u8] = asset.bytes();
    /// assert_eq!(control_data, stored_data);
    /// ```
    fn bytes(&self) -> &[u8];

    /// Transforms this content into the bytes it contains directly, taking the value without referencing.
    /// This can help avoid a .clone() for certain use cases.
    ///
    /// # Returns
    ///
    /// - `Vec<u8>` - The data this asset contains within.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::pageshelf::Asset;
    /// use pageshelf::provider::memory::MemoryAsset;
    ///
    /// let control_data: Vec<u8> = vec![1, 2, 3];
    /// let asset = MemoryAsset::from(control_data.clone());
    /// let stored_data: Vec<u8> = asset.into_bytes();
    /// assert_eq!(control_data, stored_data);
    /// ```
    fn into_bytes(self) -> Vec<u8>;

    /// The UTF-8 string that an asset contains - File content, interpreted as UTF-8.
    /// It is advised only to use this when you actually need and expect a UTF-8 string;
    /// Checks when interpreting can be expensive, and are not guaranteed to succeed.
    ///
    /// # Returns
    ///
    /// - `Result<&str, ()>` - A reference to the asset data, interpreted as a UTF-8 string.
    ///   If it could not be interpreted as a UTF-8 string, then an error.
    ///
    /// # Errors
    ///
    /// - `()` - The data could not be interpreted as a UTF-8 string.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::pageshelf::Asset;
    /// use pageshelf::provider::memory::MemoryAsset;
    ///
    /// let control_data = "Sphinx of black quartz, judge my vow".to_string();
    /// let asset = MemoryAsset::from(control_data.clone());
    /// let stored_data = asset.body().unwrap();
    /// assert_eq!(control_data, stored_data);
    /// ```
    fn body(&self) -> Result<&str, AssetError> {
        match std::str::from_utf8(self.bytes()) {
            Ok(v) => Ok(v),
            Err(_) => Err(AssetError::CannotInterpret),
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
    /// Sets the content of a given asset to match a provided asset.
    /// If the asset does not already exist, it will be created.
    ///
    /// # Arguments
    ///
    /// - `path` (`&Path`) - What asset to apply the content to
    /// - `asset` (`&impl Asset`) - The asset
    ///
    /// # Returns
    ///
    /// - `Result<(), AssetError>` - Nothing on successful operation, otherwise an error.
    ///
    /// # Errors
    ///
    /// - `ProviderError` - An internal error occurred when trying to apply the asset.
    fn set_asset(&mut self, path: &Path, asset: &impl Asset) -> Result<(), AssetError>;
    fn delete_asset(&mut self, path: &Path) -> Result<(), AssetError>;
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */
