/// In-Memory backend and tools.
///
/// This allows sourcing pages from memory; This is useful for mocking.
mod asset;

use std::{collections::HashMap, path::Path};

use crate::{
    asset::{Asset, AssetError, AssetQueryable, AssetWritable},
    page::{Page, PageError, PageSource, PageSourceFactory},
};
pub use asset::{MemoryAsset, MemoryCache};

/* -------------------------------------------------------------------------- */
/*                             Page Implementation                            */
/* -------------------------------------------------------------------------- */

struct MemoryPage<'a> {
    owner: String,
    name: String,
    branch: String,
    version: String,
    data: &'a MemoryCache,
}

impl<'a> Page for MemoryPage<'a> {
    fn branch(&self) -> &str {
        &self.branch
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn owner(&self) -> &str {
        &self.owner
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl<'a> AssetQueryable for MemoryPage<'a> {
    async fn asset_at(&self, path: &Path) -> Result<impl Asset, AssetError> {
        self.data.asset_at(path).await
    }

    fn assets(&self) -> Result<impl Iterator<Item = impl Asset>, AssetError> {
        self.data.assets()
    }
}

/* -------------------------------------------------------------------------- */
/*                        Page Provider Implementation                        */
/* -------------------------------------------------------------------------- */

#[derive(Clone)]
pub struct MemoryPageProvider {
    pages: HashMap<(String, String, String), MemoryCache>,
}

impl PageSource for MemoryPageProvider {
    async fn page_at(
        &self,
        owner: String,
        name: String,
        channel: String,
    ) -> Result<impl Page, PageError> {
        let owner = owner.to_string();
        let name = name.to_string();
        let channel = channel.to_string();
        let d = (owner.clone(), name.clone(), channel.clone());
        match self.pages.get(&d) {
            Some(v) => Ok(MemoryPage {
                owner,
                name,
                branch: channel,
                data: v,
                version: "".to_string(),
            }),
            None => Err(PageError::NotFound),
        }
    }

    async fn pages(&self) -> Result<impl Iterator<Item = impl Page>, PageError> {
        Ok(self.pages.iter().map(|f| MemoryPage {
            owner: f.0.0.clone(),
            name: f.0.1.clone(),
            branch: f.0.2.clone(),
            version: "".to_string(),
            data: &f.1,
        }))
    }
}

#[derive(Clone)]
pub struct MemoryPageProviderFactory {
    provider: MemoryPageProvider,
}

impl MemoryPageProviderFactory {
    pub fn new() -> Self {
        Self {
            provider: MemoryPageProvider {
                pages: HashMap::new(),
            },
        }
    }

    pub fn with_asset(
        mut self,
        owner: &str,
        name: &str,
        branch: &str,
        path: &Path,
        asset: MemoryAsset,
    ) -> Self {
        let id = (owner.to_string(), name.to_string(), branch.to_string());
        let page = match self.provider.pages.get_mut(&id) {
            Some(v) => v,
            None => {
                let c = MemoryCache::new();
                self.provider.pages.insert(id.clone(), c);
                self.provider.pages.get_mut(&id).unwrap()
            }
        };

        match page.write_asset(path, &asset) {
            Ok(_) => {}
            Err(e) => {
                log::error!(
                    "Error writing asset ({}) to Memory Provider: {:?}",
                    path.to_string_lossy(),
                    e
                );
            }
        }

        self
    }
}

impl PageSourceFactory for MemoryPageProviderFactory {
    type Source = MemoryPageProvider;

    fn build(&self) -> Result<Self::Source, ()> {
        Ok(self.provider.clone())
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */

pub mod testing {
    use crate::asset::Asset;

    use super::*;

    /// Ensure that the Memory Provider can create itself from a factory along with assets,
    /// then read the assets correctly.
    #[tokio::test]
    #[cfg(test)]
    async fn factory_read() {
        let p = create_example_provider();
        test_example_source(&p).await;
    }

    const OWNER_1: &str = "owner_1";
    const OWNER_2: &str = "owner_2";

    const NAME_1: &str = "name_1";
    const NAME_2: &str = "name_2";

    const BRANCH_1: &str = "pages";
    const BRANCH_2: &str = "pages";

    const DATA_1: &str = "data_1";
    const DATA_2: &str = "data_2";

    pub fn create_example_provider_factory() -> MemoryPageProviderFactory {
        let asset_path_1 = Path::new("/asset_1");
        let asset_path_2 = Path::new("/asset_2");

        let asset_1 = MemoryAsset::from_str(DATA_1);
        let asset_2 = MemoryAsset::from_str(DATA_2);

        MemoryPageProviderFactory::new()
            .with_asset(OWNER_1, NAME_1, BRANCH_1, &asset_path_1, asset_1)
            .with_asset(OWNER_2, NAME_2, BRANCH_2, &asset_path_2, asset_2)
    }

    pub fn create_example_provider() -> MemoryPageProvider {
        create_example_provider_factory().build().unwrap()
    }

    pub async fn test_example_source(p: &MemoryPageProvider) {
        let asset_path_1 = Path::new("/asset_1");
        let asset_path_2 = Path::new("/asset_2");

        assert_eq!(p.pages().await.unwrap().count(), 2);

        let page_1 = p
            .page_at(
                OWNER_1.to_string(),
                NAME_1.to_string(),
                BRANCH_1.to_string(),
            )
            .await
            .unwrap();
        let page_2 = p
            .page_at(
                OWNER_2.to_string(),
                NAME_2.to_string(),
                BRANCH_2.to_string(),
            )
            .await
            .unwrap();

        // Validate asset count
        assert_eq!(page_1.assets().unwrap().count(), 1);
        assert_eq!(page_2.assets().unwrap().count(), 1);

        // Validate asset accessing
        assert_eq!(page_1.asset_at(&asset_path_1).await.unwrap().body(), DATA_1);
        assert_eq!(page_2.asset_at(&asset_path_2).await.unwrap().body(), DATA_2);
        assert!(page_1.asset_at(&asset_path_2).await.is_err());
        assert!(page_2.asset_at(&asset_path_1).await.is_err());

        // Validate incorrect page accessing
        assert!(
            p.page_at(
                OWNER_2.to_string(),
                NAME_1.to_string(),
                BRANCH_1.to_string()
            )
            .await
            .is_err()
        );
        assert!(
            p.page_at(
                OWNER_2.to_string(),
                NAME_1.to_string(),
                BRANCH_2.to_string()
            )
            .await
            .is_err()
        );
        assert!(
            p.page_at(
                OWNER_1.to_string(),
                NAME_2.to_string(),
                BRANCH_1.to_string()
            )
            .await
            .is_err()
        );
        assert!(
            p.page_at(
                OWNER_1.to_string(),
                NAME_2.to_string(),
                BRANCH_2.to_string()
            )
            .await
            .is_err()
        );
        if BRANCH_1 != BRANCH_2 {
            assert!(
                p.page_at(
                    OWNER_1.to_string(),
                    NAME_1.to_string(),
                    BRANCH_2.to_string()
                )
                .await
                .is_err()
            );
            assert!(
                p.page_at(
                    OWNER_2.to_string(),
                    NAME_1.to_string(),
                    BRANCH_2.to_string()
                )
                .await
                .is_err()
            );
            assert!(
                p.page_at(
                    OWNER_2.to_string(),
                    NAME_2.to_string(),
                    BRANCH_1.to_string()
                )
                .await
                .is_err()
            );
        } else {
            assert!(
                p.page_at(
                    OWNER_1.to_string(),
                    NAME_1.to_string(),
                    BRANCH_2.to_string()
                )
                .await
                .is_ok()
            );
            assert!(
                p.page_at(
                    OWNER_2.to_string(),
                    NAME_2.to_string(),
                    BRANCH_1.to_string()
                )
                .await
                .is_ok()
            );
            assert!(
                p.page_at(
                    OWNER_2.to_string(),
                    NAME_1.to_string(),
                    BRANCH_2.to_string()
                )
                .await
                .is_err()
            );
        }
    }
}
