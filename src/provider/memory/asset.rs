/// In-memory implementation of Assets and asset storage
///
/// It will simply show what is stored in memory inside it. Useful for mocking.
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use log::info;

use crate::{Asset, AssetError, AssetSource, AssetWritable};

/// An Asset that is stored and accessed from memory.
#[derive(Clone)]
pub struct MemoryAsset {
    contents: Vec<u8>,
}

impl MemoryAsset {
    pub fn new_from_bytes(data: Vec<u8>) -> Self {
        Self { contents: data }
    }

    pub fn new_from_str(data: &str) -> Self {
        Self {
            contents: data.into(),
        }
    }

    pub fn empty() -> Self {
        Self { contents: vec![] }
    }
}

impl Asset for MemoryAsset {
    fn into_bytes(self) -> Vec<u8> {
        self.contents
    }
    fn bytes(&self) -> &[u8] {
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
    fn into_bytes(self) -> Vec<u8> {
        self.asset.bytes().to_vec()
    }
    fn bytes(&self) -> &[u8] {
        self.asset.bytes()
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

impl Default for MemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetSource for MemoryCache {
    async fn get_asset(&self, path: &Path) -> Result<impl Asset, AssetError> {
        let buf = std::path::absolute(Path::new("/").join(path)).unwrap();
        info!("Getting MemoryAsset {:?}...", buf);
        match self.data.get(&buf) {
            Some(v) => Ok(AssetRef::new(v)),
            None => Err(AssetError::NotFound),
        }
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

    fn set_asset(&mut self, path: &Path, asset: &impl Asset) -> Result<(), AssetError> {
        self.data.insert(
            path.to_path_buf(),
            MemoryAsset {
                contents: asset.bytes().to_vec(),
            },
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Asset;

    use super::MemoryAsset;

    /// Can we get the same string back from the asset?
    #[test]
    fn memory_asset_str() {
        let data: Vec<u8> = vec![b'm', b'e', b'o', b'w'];
        let asset = MemoryAsset::new_from_bytes(data.clone());

        assert_eq!(asset.bytes(), data)
    }
}
