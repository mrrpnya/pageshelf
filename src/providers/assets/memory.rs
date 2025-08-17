/// In-memory implementation of an
/// It will simply show pages that are stored in memory inside it.

use std::collections::HashMap;

use crate::asset::{Asset, AssetError, AssetPath, AssetQueryable, AssetWritable};

#[derive(Clone)]
pub struct MemoryAsset {
    contents: Vec<u8>,
}

impl MemoryAsset {
    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            contents: Vec::from(data),
        }
    }

    pub fn from_str(data: &str) -> Self {
        Self {
            contents: data.as_bytes().to_vec(),
        }
    }
}

impl Asset for MemoryAsset {
    fn body(&self) -> String {
        unsafe { String::from_utf8_unchecked(self.contents.clone()) }
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

#[derive(Clone)]
pub struct MemoryCache {
    data: HashMap<String, MemoryAsset>,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self {
            data: HashMap::new()
        }
    }
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
    use actix_web::body::MessageBody;

    use crate::asset::Asset;

    use super::MemoryAsset;

    #[test]
    fn memory_asset_bytes() {
        let data = [8; 8];
        let asset = MemoryAsset::from_bytes(&data);

        assert!(asset.bytes().count() == data.iter().count())
    }

    #[test]
    fn memory_asset_str() {
        let data = "meow";
        let asset = MemoryAsset::from_str(data);

        assert_eq!(asset.body(), data)
    }
}
