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
    fn hash_sha256(&self) -> [u8; 32] {
        // TODO: Calculate SHA256 from .bytes()
        [0; 32]
    }
}

pub trait AssetQueryable {
    async fn asset_at(&self, path: &Path) -> Result<impl Asset, AssetError>;
    fn assets(&self) -> Result<impl Iterator<Item = impl Asset>, AssetError>;
    fn total_bytes(&self) -> Option<u32> {
        None
    }
}

pub trait AssetWritable {
    fn write_asset(&mut self, path: &Path, asset: &impl Asset) -> Result<(), AssetError>;
    fn delete_asset(&mut self, path: &Path) -> Result<(), AssetError>;
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */
