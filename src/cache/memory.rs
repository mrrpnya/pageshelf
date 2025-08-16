use std::collections::HashMap;

use crate::asset::{Asset, AssetQueryable};

struct MemoryCacheAsset {
    contents: Vec<u8>,
    hash: u128
}

impl Asset for MemoryCacheAsset {
    fn body(&self) -> &str {
        self.contents
    }

    fn bytes(&self) -> impl Iterator<Item = u8> {
        self.contents.iter()
    }
}

pub struct MemoryCache {
    data: HashMap<String, MemoryCacheAsset>
}

impl AssetQueryable for MemoryCache {
    fn asset_at(&self, route: &str) -> Result<impl Asset, PageError> {
        todo!()
    }

    fn assets(&self) -> impl Iterator<Item = impl Asset> {
        todo!()
    }
}