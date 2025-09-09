/// Utilities for sourcing pages from Forgejo directly, via raw file access.
use std::path::Path;

use forgejo_api::{Forgejo, structs::RepoGetRawFileQuery};
use log::{error, info};

use crate::{Asset, AssetError, AssetSource};

use crate::provider::memory::MemoryAsset;

pub struct ForgejoDirectReadStorage<'a> {
    forgejo: &'a Forgejo,
    owner: String,
    repo: String,
    branch: String,
    version: String,
}

impl<'a> ForgejoDirectReadStorage<'a> {
    pub fn new(
        forgejo: &'a Forgejo,
        owner: String,
        repo: String,
        branch: String,
        version: String,
    ) -> Self {
        Self {
            forgejo,
            owner,
            repo,
            branch,
            version,
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repo(&self) -> &str {
        &self.repo
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

impl<'a> AssetSource for ForgejoDirectReadStorage<'a> {
    async fn get_asset(&self, path: &Path) -> Result<impl Asset, AssetError> {
        let p = path.to_string_lossy();
        info!("Fetching Forgejo raw data at {}", p);
        match self
            .forgejo
            .repo_get_raw_file(
                self.owner.as_str(),
                self.repo.as_str(),
                &p,
                RepoGetRawFileQuery {
                    r#ref: Some(self.branch.clone()),
                },
            )
            .await
        {
            Ok(v) => Ok(MemoryAsset::new_from_bytes(v)),
            Err(e) => {
                error!(
                    "Failed to find (raw) data file {} in Forgejo repository {}/{}:{} - {}",
                    path.to_string_lossy(),
                    self.owner,
                    self.repo,
                    self.branch,
                    e
                );
                Err(AssetError::NotFound)
            }
        }
    }
}
