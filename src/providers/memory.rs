use std::collections::HashMap;

use crate::{asset::AssetQueryable, storage::memory::MemoryCache, Page, PageError, PageSource};

struct MemoryPage<'a> {
    owner: String,
    name: String,
    branch: String,
    data: &'a MemoryCache
}

impl<'a> Page for MemoryPage<'a> {
    fn channel(&self) -> &str {
        &self.branch
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn owner(&self) -> &str {
        &self.owner
    }
}

impl<'a> AssetQueryable for MemoryPage<'a> {
    async fn asset_at(&self, path: &crate::asset::AssetPath) -> Result<impl crate::asset::Asset, crate::asset::AssetError> {
        self.data.asset_at(path).await
    }

    fn assets(&self) -> Result<impl Iterator<Item = impl crate::asset::Asset>, crate::asset::AssetError> {
        self.data.assets()
    }
}

pub struct MemoryPageProvider {
    pages: HashMap<(String, String, String), MemoryCache>
}

impl PageSource for MemoryPageProvider {
    async fn page_at(
        &self,
        owner: &str,
        name: &str,
        channel: &str,
    ) -> Result<impl Page, crate::PageError> {
        let owner = owner.to_string();
        let name = name.to_string();
        let channel = channel.to_string();
        let d = (owner.clone(), name.clone(), channel.clone());
        match self.pages.get(&d) {
            Some(v) => {
                Ok(MemoryPage {
                    owner,
                    name,
                    branch: channel,
                    data: v
                })
            }
            None => Err(PageError::NotFound)
        }
    }

    async fn pages(&self) -> Result<impl Iterator<Item = impl Page>, crate::PageError> {
        Ok(self.pages.iter().map(|f| MemoryPage {
            owner: f.0.0.clone(),
            name: f.0.1.clone(),
            branch: f.0.2.clone(),
            data: &f.1
        }))
    }
}