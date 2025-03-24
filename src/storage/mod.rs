pub mod backend_filesystem;
pub mod gitea_filesystem;

pub enum PageStorageError {
    SiteDoesNotExist(String),
    AssetDoesNotExist(String),
    InternalError(String)
}

pub enum PageStorageAssetType {
    IsNone,
    IsFile,
    IsDirectory
}

pub trait PageStorageRead {
    fn asset_contents(&self, site_id: &str, url: &str) -> Result<Vec<u8>, PageStorageError>;
    fn asset_exists(&self, site_id: &str, url: &str) -> Result<PageStorageAssetType, PageStorageError>;
    fn site_exists(&self, site_id: &str) -> Result<bool, PageStorageError>;
}