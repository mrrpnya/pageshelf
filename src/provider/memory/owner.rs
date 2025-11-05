use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{
    project::ProjectOwner,
    provider::memory::{MemoryAsset, project::MemoryProject},
};

#[derive(Clone)]
pub struct MemoryProjectOwner {
    name: String,
    projects: HashMap<String, MemoryProject>,
}

impl MemoryProjectOwner {
    pub fn empty(name: String) -> Self {
        Self {
            name,
            projects: HashMap::new(),
        }
    }

    pub fn new(name: String, projects: HashMap<String, MemoryProject>) -> Self {
        Self { name, projects }
    }

    pub fn insert_asset(&mut self, project: &str, channel: &str, path: &Path, asset: MemoryAsset) {
        match self.projects.get_mut(project) {
            Some(project) => {
                project.insert_asset(channel, path, asset);
            }
            None => {
                self.projects.insert(
                    project.to_string(),
                    MemoryProject::empty(project.to_string(), Some("pages".to_string()))
                        .with_asset(channel, path, asset),
                );
            }
        };
    }

    pub fn with_asset(
        mut self,
        project: &str,
        channel: &str,
        path: &Path,
        asset: MemoryAsset,
    ) -> Self {
        self.insert_asset(project, channel, path, asset);

        self
    }
}

impl ProjectOwner for MemoryProjectOwner {
    type Project<'b>
        = &'b MemoryProject
    where
        Self: 'b;

    fn name(&self) -> &str {
        &self.name
    }

    async fn projects<'b>(
        &'b self,
    ) -> Result<impl Iterator<Item = Self::Project<'b>> + 'b, crate::project::ProjectError> {
        Ok(self.projects.iter().map(|f| f.1))
    }
}

impl ProjectOwner for &MemoryProjectOwner {
    type Project<'b>
        = &'b MemoryProject
    where
        Self: 'b;

    fn name(&self) -> &str {
        &self.name
    }

    async fn projects<'b>(
        &'b self,
    ) -> Result<impl Iterator<Item = Self::Project<'b>> + 'b, crate::project::ProjectError> {
        Ok(self.projects.iter().map(|f| f.1))
    }
}
