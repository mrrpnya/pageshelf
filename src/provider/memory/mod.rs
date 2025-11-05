mod owner;
mod page;
mod project;
use std::{collections::HashMap, path::Path, sync::Arc};

use owner::*;
use page::*;
use project::*;

/// In-Memory backend and tools.
///
/// This allows sourcing pages from memory; This is useful for mocking.
mod asset;

pub use asset::{MemoryAsset, MemoryCache};

use crate::project::{ProjectOwner, source::ProjectSource};

/* -------------------------------------------------------------------------- */
/*                        Page Provider Implementation                        */
/* -------------------------------------------------------------------------- */

#[derive(Clone, Default)]
pub struct MemoryProjectSource {
    owners: HashMap<String, MemoryProjectOwner>,
}

impl MemoryProjectSource {
    pub fn insert_asset(
        &mut self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
        asset: MemoryAsset,
    ) {
        match self.owners.get_mut(owner) {
            Some(owner) => {
                owner.insert_asset(project, channel, path, asset);
            }
            None => {
                self.owners.insert(
                    owner.to_string(),
                    MemoryProjectOwner::empty(owner.to_string())
                        .with_asset(project, channel, path, asset),
                );
            }
        }
    }

    pub fn with_asset(
        mut self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
        asset: MemoryAsset,
    ) -> Self {
        self.insert_asset(owner, project, channel, path, asset);
        self
    }

    pub fn with_owner(mut self, owner: MemoryProjectOwner) -> Self {
        self.owners.insert(owner.name().to_string(), owner);

        self
    }
}

impl ProjectSource for MemoryProjectSource {
    type Owner<'b>
        = &'b MemoryProjectOwner
    where
        Self: 'b;

    async fn all_owners<'b>(
        &'b self,
    ) -> Result<impl Iterator<Item = Self::Owner<'b>>, crate::project::ProjectError> {
        Ok(self.owners.iter().map(|f| f.1))
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */

pub mod testing {
    use std::path::Path;

    use super::*;

    const OWNER_1: &str = "owner_1";
    const OWNER_2: &str = "owner_2";

    const NAME_1: &str = "name_1";
    const NAME_2: &str = "name_2";

    const BRANCH_1: &str = "pages";
    const BRANCH_2: &str = "pages";

    const DATA_1: &str = "data_1";
    const DATA_2: &str = "data_2";

    pub fn create_example_provider() -> MemoryProjectSource {
        let asset_path_1 = Path::new("/asset_1");
        let asset_path_2 = Path::new("/asset_2");

        let asset_1 = MemoryAsset::from(DATA_1);
        let asset_2 = MemoryAsset::from(DATA_2);

        MemoryProjectSource::default()
            .with_asset(OWNER_1, NAME_1, BRANCH_1, asset_path_1, asset_1)
            .with_asset(OWNER_2, NAME_2, BRANCH_2, asset_path_2, asset_2)
    }
    /*
    pub async fn test_example_source(p: &MemoryPageProvider) {
        let asset_path_1 = Path::new("/asset_1");
        let asset_path_2 = Path::new("/asset_2");

        assert_eq!(p.pages().await.unwrap().count(), 2);

        let page_1 = p
            .get_page(
                OWNER_1.to_string(),
                NAME_1.to_string(),
                BRANCH_1.to_string(),
            )
            .await
            .unwrap();
        let page_2 = p
            .get_page(
                OWNER_2.to_string(),
                NAME_2.to_string(),
                BRANCH_2.to_string(),
            )
            .await
            .unwrap();

        // Validate asset accessing
        assert_eq!(
            page_1
                .get_asset(asset_path_1)
                .await
                .unwrap()
                .body()
                .unwrap(),
            DATA_1
        );
        assert_eq!(
            page_2
                .get_asset(asset_path_2)
                .await
                .unwrap()
                .body()
                .unwrap(),
            DATA_2
        );
        assert!(page_1.get_asset(asset_path_2).await.is_err());
        assert!(page_2.get_asset(asset_path_1).await.is_err());

        // Validate incorrect page accessing
        assert!(
            p.get_page(
                OWNER_2.to_string(),
                NAME_1.to_string(),
                BRANCH_1.to_string()
            )
            .await
            .is_err()
        );
        assert!(
            p.get_page(
                OWNER_2.to_string(),
                NAME_1.to_string(),
                BRANCH_2.to_string()
            )
            .await
            .is_err()
        );
        assert!(
            p.get_page(
                OWNER_1.to_string(),
                NAME_2.to_string(),
                BRANCH_1.to_string()
            )
            .await
            .is_err()
        );
        assert!(
            p.get_page(
                OWNER_1.to_string(),
                NAME_2.to_string(),
                BRANCH_2.to_string()
            )
            .await
            .is_err()
        );
        if BRANCH_1 != BRANCH_2 {
            assert!(
                p.get_page(
                    OWNER_1.to_string(),
                    NAME_1.to_string(),
                    BRANCH_2.to_string()
                )
                .await
                .is_err()
            );
            assert!(
                p.get_page(
                    OWNER_2.to_string(),
                    NAME_1.to_string(),
                    BRANCH_2.to_string()
                )
                .await
                .is_err()
            );
            assert!(
                p.get_page(
                    OWNER_2.to_string(),
                    NAME_2.to_string(),
                    BRANCH_1.to_string()
                )
                .await
                .is_err()
            );
        } else {
            assert!(
                p.get_page(
                    OWNER_1.to_string(),
                    NAME_1.to_string(),
                    BRANCH_2.to_string()
                )
                .await
                .is_ok()
            );
            assert!(
                p.get_page(
                    OWNER_2.to_string(),
                    NAME_2.to_string(),
                    BRANCH_1.to_string()
                )
                .await
                .is_ok()
            );
            assert!(
                p.get_page(
                    OWNER_2.to_string(),
                    NAME_1.to_string(),
                    BRANCH_2.to_string()
                )
                .await
                .is_err()
            );
        }
    }
    */
}
