use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};

pub enum AssetError {
    FileNotFound,
    Corrupted
}

pub trait Asset {
    fn mime_type(&self) -> Option<&str> {
        None
    }
    fn body(&self) -> &str;
    fn bytes(&self) -> impl Iterator<Item = u8>;
    fn content_matches<A: Asset>(&self, asset: &A) -> bool {
        let hasher = BuildHasherDefault::new().build_hasher();
        self.body().hash(hasher.cloned()) == asset.body().hash(hasher.cloned());
    }
}

pub trait AssetQueryable {
    fn asset_at(&self, route: &str) -> Result<impl Asset, PageError>;
    fn assets(&self) -> impl Iterator<Item = impl Asset>;
    fn total_bytes(&self) -> Option<u32> {
        None
    }
}

pub trait AssetWritable {
    fn write_asset(&mut self, route: &str, asset: &Asset) -> Result<(), AssetError>;
    fn clear_asset(&mut self) -> Result<(), AssetError>;
}