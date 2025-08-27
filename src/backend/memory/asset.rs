/// In-memory implementation of Assets and asset storage
///
/// It will simply show what is stored in memory inside it. Useful for mocking.
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use log::info;

use crate::asset::{Asset, AssetError, AssetQueryable, AssetWritable};

/// An Asset that is stored and accessed from memory.
#[derive(Clone)]
pub struct MemoryAsset {
    contents: String,
}

impl MemoryAsset {
    pub fn from_bytes(data: Vec<u8>) -> Self {
        unsafe {
            Self {
                contents: String::from_utf8_unchecked(data).to_string(),
            }
        }
    }

    pub fn from_str(data: &str) -> Self {
        Self {
            contents: data.to_string(),
        }
    }

    pub fn empty() -> Self {
        Self {
            contents: "".to_string(),
        }
    }
}

impl Asset for MemoryAsset {
    fn body(&self) -> &str {
        &self.contents
    }
}

pub struct AssetRef<'a, A: Asset> {
    asset: &'a A,
}

impl<'a, A: Asset> AssetRef<'a, A> {
    pub fn new(asset: &'a A) -> Self {
        Self { asset }
    }
}

impl<'a, A: Asset> Asset for AssetRef<'a, A> {
    fn body(&self) -> &str {
        self.asset.body()
    }
}

/// A group of assets that are stored in memory and can be accessed.
#[derive(Clone)]
pub struct MemoryCache {
    data: HashMap<PathBuf, MemoryAsset>,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl AssetQueryable for MemoryCache {
    async fn asset_at(&self, path: &Path) -> Result<impl Asset, AssetError> {
        let buf = std::path::absolute(Path::new("/").join(path.to_path_buf())).unwrap();
        info!("Getting MemoryAsset {:?}...", buf);
        match self.data.get(&buf) {
            Some(v) => Ok(AssetRef::new(v)),
            None => Err(AssetError::NotFound),
        }
    }

    fn assets(&self) -> Result<impl Iterator<Item = impl Asset>, AssetError> {
        Ok(self.data.values().map(|f| AssetRef::new(f)))
    }
}

impl AssetWritable for MemoryCache {
    fn delete_asset(&mut self, path: &Path) -> Result<(), AssetError> {
        let buf = path.to_path_buf();
        match self.data.remove(&buf) {
            Some(_) => Ok(()),
            None => Err(AssetError::NotFound),
        }
    }

    fn write_asset(&mut self, path: &Path, asset: &impl Asset) -> Result<(), AssetError> {
        self.data.insert(
            path.to_path_buf(),
            MemoryAsset {
                contents: asset.body().to_string(),
            },
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::asset::Asset;

    use super::MemoryAsset;

    /// Can we get the same string back from the asset?
    #[test]
    fn memory_asset_str() {
        let data = "meow";
        let asset = MemoryAsset::from_str(data);

        assert_eq!(asset.body(), data)
    }
}
