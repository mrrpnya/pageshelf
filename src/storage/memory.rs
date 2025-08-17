use std::collections::HashMap;

use crate::asset::{Asset, AssetError, AssetPath, AssetQueryable, AssetWritable};

pub struct MemoryAsset {
    contents: Vec<u8>,
}

impl MemoryAsset {
    pub fn new(data: &[u8]) -> Self {
        Self {
            contents: Vec::from(data),
        }
    }
}

impl Asset for MemoryAsset {
    fn body(&self) -> String {
        String::from_utf8(self.contents.clone()).unwrap()
    }

    fn bytes(&self) -> impl Iterator<Item = u8> {
        self.contents.iter().cloned()
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
    fn body(&self) -> String {
        self.asset.body()
    }
    fn bytes(&self) -> impl Iterator<Item = u8> {
        self.asset.bytes()
    }
}

pub struct MemoryCache {
    data: HashMap<String, MemoryAsset>,
}

impl AssetQueryable for MemoryCache {
    async fn asset_at(&self, path: &AssetPath) -> Result<impl Asset, AssetError> {
        match self.data.get(path.path()) {
            Some(v) => Ok(AssetRef::new(v)),
            None => Err(AssetError::NotFound),
        }
    }

    fn assets(&self) -> Result<impl Iterator<Item = impl Asset>, AssetError> {
        Ok(self.data.values().map(|f| AssetRef::new(f)))
    }
}

impl AssetWritable for MemoryCache {
    fn delete_asset(&mut self, path: &AssetPath) -> Result<(), AssetError> {
        match self.data.remove(path.path()) {
            Some(_) => Ok(()),
            None => Err(AssetError::NotFound),
        }
    }

    fn write_asset(&mut self, path: &AssetPath, asset: &impl Asset) -> Result<(), AssetError> {
        self.data.insert(
            path.to_string(),
            MemoryAsset {
                contents: asset.bytes().collect(),
            },
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::asset::Asset;

    use super::MemoryAsset;

    #[test]
    fn memory_asset() {
        let data = [8; 8];
        let asset = MemoryAsset::new(&data);

        assert!(asset.bytes().count() == data.iter().count())
    }
}