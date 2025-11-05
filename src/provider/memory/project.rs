use std::{collections::HashMap, path::Path};

use crate::{
    project::{Project, ProjectError},
    provider::memory::{MemoryAsset, MemoryPage},
};

#[derive(Clone)]
pub struct MemoryProject {
    name: String,
    default_channel: Option<String>,
    pages: HashMap<String, MemoryPage>,
}

impl MemoryProject {
    pub fn empty(name: String, default_channel: Option<String>) -> Self {
        Self {
            name,
            default_channel,
            pages: HashMap::new(),
        }
    }

    pub fn insert_asset(&mut self, channel: &str, path: &Path, asset: MemoryAsset) {
        match self.pages.get_mut(channel) {
            Some(channel) => {
                channel.insert_asset(path, asset);
            }
            None => {
                self.pages.insert(
                    channel.to_string(),
                    MemoryPage::empty(channel.to_string()).with_asset(path, asset),
                );
            }
        }
    }

    pub fn with_asset(mut self, channel: &str, path: &Path, asset: MemoryAsset) -> Self {
        self.insert_asset(channel, path, asset);
        self
    }
}

impl Project for MemoryProject {
    type Page<'b>
        = &'b MemoryPage
    where
        Self: 'b;

    type Error = ProjectError;

    fn name(&self) -> &str {
        &self.name
    }

    fn default_channel(&self) -> Option<&str> {
        match &self.default_channel {
            Some(v) => Some(v.as_str()),
            None => None,
        }
    }

    async fn channels<'b>(
        &'b self,
    ) -> Result<impl Iterator<Item = Self::Page<'b>> + 'b, Self::Error> {
        Ok(self.pages.values())
    }
}

impl Project for &MemoryProject {
    type Page<'b>
        = &'b MemoryPage
    where
        Self: 'b;

    type Error = ProjectError;

    fn name(&self) -> &str {
        &self.name
    }

    fn default_channel(&self) -> Option<&str> {
        match &self.default_channel {
            Some(v) => Some(v.as_str()),
            None => None,
        }
    }

    async fn channels<'b>(
        &'b self,
    ) -> Result<impl Iterator<Item = Self::Page<'b>> + 'b, Self::Error> {
        Ok(self.pages.values())
    }
}
