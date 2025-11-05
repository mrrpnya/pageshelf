use std::{cell::RefCell, path::Path, sync::Arc};

use crate::{
    Asset, AssetError, AssetSource,
    ext::Normalizable,
    project::{Page, Project, ProjectOwner},
    provider::{forgejo::project::ForgejoProject, memory::MemoryAsset},
};

use forgejo_api::structs::RepoGetRawFileQuery;
use tokio::sync::RwLock;
use tracing::{error, info};

pub struct ForgejoPage<'a> {
    pub project: &'a ForgejoProject<'a>,
    pub domains: Arc<RwLock<Option<Vec<String>>>>,
    id: String,
    version: String,
}

impl<'a> ForgejoPage<'a> {
    pub fn new(project: &'a ForgejoProject, id: String, version: String) -> Self {
        Self {
            project,
            id,
            domains: Arc::new(RwLock::new(None)),
            version,
        }
    }
}

impl<'a> Page for ForgejoPage<'a> {
    fn name(&self) -> &str {
        &self.id
    }
    fn version(&self) -> &str {
        &self.version
    }
}

impl<'a> AssetSource for ForgejoPage<'a> {
    async fn get_asset(&self, path: &Path) -> Result<impl Asset, AssetError> {
        let normalized = path.normalized();
        let p = normalized.to_string_lossy();
        info!("Fetching Forgejo raw data at {}", p);
        match self
            .project
            .owner
            .forgejo
            .repo_get_raw_file(
                self.project.owner.name(),
                self.project.name(),
                &p,
                RepoGetRawFileQuery {
                    r#ref: Some(self.id.clone()),
                },
            )
            .await
        {
            Ok(v) => Ok(MemoryAsset::from(v)),
            Err(e) => {
                error!(
                    "Failed to find (raw) data file {} in Forgejo repository {}/{}:{} - {}",
                    path.to_string_lossy(),
                    self.project.owner.name(),
                    self.project.name(),
                    self.name(),
                    e
                );
                Err(AssetError::NotFound)
            }
        }
    }

    async fn asset_keys(&self) -> Result<impl Iterator<Item = String>, AssetError> {
        return Err(AssetError::NotImplemented);
        // Unreachable, but here for type hinting
        Ok(Vec::new().into_iter())
    }
}
