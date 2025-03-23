

pub trait PageStorageRead {
    fn url_contents(site_id: &str, url: &str) -> Result<[u8], ()>;
    fn url_exists(site_id: &str, url: &str) -> Result<bool, ()>;
}

pub trait PageStorageWrite {
    
}