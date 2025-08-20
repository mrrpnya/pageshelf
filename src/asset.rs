use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum AssetError {
    NotFound,
    Corrupted,
    ProviderError,
}
/// Represents a file in a page.
pub trait Asset {
    fn mime_type(&self) -> Option<&str> {
        None
    }
    fn body(&self) -> &str;
}

/// A trait that allows finding assets.
pub trait AssetQueryable {
    async fn asset_at(&self, path: &Path) -> Result<impl Asset, AssetError>;
    fn assets(&self) -> Result<impl Iterator<Item = impl Asset>, AssetError>;
    fn total_bytes(&self) -> Option<u32> {
        None
    }
}

/// A trait that enables manipulation of assets on its implementors.
pub trait AssetWritable {
    fn write_asset(&mut self, path: &Path, asset: &impl Asset) -> Result<(), AssetError>;
    fn delete_asset(&mut self, path: &Path) -> Result<(), AssetError>;
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */
