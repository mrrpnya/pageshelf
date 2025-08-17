use forgejo_api::{Forgejo, structs::RepoGetRawFileQuery};
use log::{error, warn};

use crate::asset::{Asset, AssetError, AssetPath, AssetQueryable};

use super::memory::MemoryAsset;

pub struct ForgejoDirectReadStorage<'a> {
    forgejo: &'a Forgejo,
    owner: String,
    repo: String,
    branch: String,
}

struct EmptyAssetIter {}

impl EmptyAssetIter {
    fn new() -> Self {
        Self {}
    }
}

impl Iterator for EmptyAssetIter {
    type Item = MemoryAsset;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

impl<'a> ForgejoDirectReadStorage<'a> {
    pub fn new(forgejo: &'a Forgejo, owner: String, repo: String, branch: String) -> Self {
        Self {
            forgejo,
            owner,
            repo,
            branch,
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
}

impl<'a> AssetQueryable for ForgejoDirectReadStorage<'a> {
    async fn asset_at(&self, path: &AssetPath) -> Result<impl Asset, AssetError> {
        let p = path.to_string();
        match self
            .forgejo
            .repo_get_raw_file(
                self.owner.as_str(),
                self.repo.as_str(),
                p.as_str(),
                RepoGetRawFileQuery {
                    r#ref: Some(self.branch.clone()),
                },
            )
            .await
        {
            Ok(v) => Ok(MemoryAsset::new(&v)),
            Err(e) => {
                error!(
                    "Failed to find (raw) data file {} in Forgejo repository {}/{}:{} - {}",
                    path.to_string(),
                    self.owner,
                    self.repo,
                    self.branch,
                    e
                );
                Err(AssetError::NotFound)
            }
        }
    }

    fn assets(&self) -> Result<impl Iterator<Item = impl Asset>, AssetError> {
        warn!("Iteration of Forgejo files is not implemented");
        Ok(EmptyAssetIter::new())
    }
}
