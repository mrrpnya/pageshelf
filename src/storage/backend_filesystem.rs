use std::{fs, io::Read, path::{self, Path}};

use log::debug;

use super::{PageStorageAssetType, PageStorageError, PageStorageRead};

#[derive(Debug)]
pub enum PageStorageBackendFilesystemError {
    InvalidDirectory(String)
}

pub struct PageStorageBackendFilesystem {
    storage_directory: String
}

impl PageStorageBackendFilesystem {
    pub fn new(storage_directory: String) -> Result<Self, PageStorageBackendFilesystemError> {
        // TODO: Perform checks on storage directory.
        // - Is it a valid directory?
        // - Is it a system directory?
        Ok(Self {
            storage_directory
        })
    }
}

// Stores page assets on filesystem as the following layout:
// - page_data
//   - [...site_id...]
//     - [...site assets...]
// ! WARNING: THIS IS NOT CURRENTLY ROBUST ENOUGH FOR PRODUCTION
impl PageStorageRead for PageStorageBackendFilesystem {
    fn asset_contents(&self, site_id: &str, url: &str) -> Result<Vec<u8>, super::PageStorageError> {
        match self.asset_exists(site_id, url) {
            Ok(exists) => {
                let mut url: String = url.to_string();
                match exists {
                    PageStorageAssetType::IsFile => {},
                    PageStorageAssetType::IsDirectory => {
                        debug!("Requested asset is a directory: Inferring index.html");
                        url = format!("{}/index.html", url);
                    }
                    PageStorageAssetType::IsNone => return Err(PageStorageError::AssetDoesNotExist(url.to_string()))
                }
                // TODO: Reprocessing the path - Make it only do it once
                let path_raw = format!("{}/{}/{}", &self.storage_directory, site_id, url);
                let path: &Path = Path::new(&path_raw);

                let file_result = fs::File::open(path);
                match file_result {
                    Ok(mut file) => {
                        let mut vec = Vec::new();
                        // TODO: Unused result
                        file.read_to_end(&mut vec);
                        return Ok(vec)
                    }
                    Err(_) => Err(super::PageStorageError::InternalError("".to_string()))
                }
            },
            Err(e) => Err(e)
        }
    }

    fn asset_exists(&self, site_id: &str, url: &str) -> Result<PageStorageAssetType, super::PageStorageError> {
        match self.site_exists(site_id) {
            Ok(exists) => {
                match exists {
                    true => {}
                    false => return Err(PageStorageError::SiteDoesNotExist(site_id.to_string()))
                }
                // TODO: VALIDATE URL
                // TODO: We're reprocessing the path here - Make it only do it once
                let path_raw = format!("{}/{}/{}", &self.storage_directory, site_id, url);
                let path: &Path = Path::new(&path_raw);
                if !path.exists() {
                    debug!("Asset {} does not exist", url);
                    return Ok(PageStorageAssetType::IsNone)
                } if path.is_file() {
                    debug!("Asset {} is a file", url);
                    return Ok(PageStorageAssetType::IsFile)
                } else if path.is_dir() {
                    debug!("Asset {} is a directory", url);
                    return Ok(PageStorageAssetType::IsDirectory)
                } else {
                    return Err(PageStorageError::InternalError("Page exists, but is neither a file nor directory".to_string()))
                }
            },
            Err(e) => Err(e)
        }
    }

    fn site_exists(&self, site_id: &str) -> Result<bool, super::PageStorageError> {
        // TODO: VALIDATE SITE_ID
        let path_raw = format!("{}/{}", &self.storage_directory, site_id);
        let path: &Path = Path::new(&path_raw);
        let exists = path.exists() && path.is_dir();
        debug!("Checking if site {} exists: {}", site_id, exists);
        Ok(exists)
    }
}