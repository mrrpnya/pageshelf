use std::{collections::HashMap, path::Path};

use crate::{
    Asset, AssetError, AssetSource,
    ext::Normalizable,
    project::Page,
    provider::memory::{MemoryAsset, MemoryCache},
};

#[derive(Clone)]
pub struct MemoryPage {
    name: String,
    version: String,
    domains: Vec<String>,
    data: HashMap<String, MemoryAsset>,
}

impl MemoryPage {
    pub fn empty(name: String) -> Self {
        Self {
            name,
            version: "".to_string(),
            domains: Vec::new(),
            data: HashMap::new(),
        }
    }

    pub fn insert_asset(&mut self, path: &Path, asset: MemoryAsset) {
        match self.data.get_mut(path.to_str().unwrap()) {
            Some(a) => {
                *a = asset;
            }
            None => {
                self.data
                    .insert(path.normalized().to_str().unwrap().to_string(), asset);
            }
        }
    }

    pub fn with_asset(mut self, path: &Path, asset: MemoryAsset) -> Self {
        self.insert_asset(path, asset);
        self
    }
}

impl<'a> Page for &'a MemoryPage {
    fn name(&self) -> &str {
        &self.name
    }
    fn version(&self) -> &str {
        &self.version
    }
}

impl AssetSource for &MemoryPage {
    async fn get_asset(&self, path: &Path) -> Result<impl Asset, AssetError> {
        match self.data.get(path.normalized().to_str().unwrap()) {
            Some(v) => Ok(v),
            None => Err(AssetError::NotFound),
        }
    }

    async fn asset_keys(&self) -> Result<impl Iterator<Item = String>, AssetError> {
        Ok(self.data.keys().map(|f| f.to_string()))
    }
}
