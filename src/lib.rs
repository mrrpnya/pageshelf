use asset::AssetQueryable;

pub mod asset;
pub mod providers;
pub mod routes;
pub mod storage;
pub mod templates;
pub mod conf;

#[derive(Debug, PartialEq, Eq)]
pub enum PageError {
    NotFound,
    ProviderError,
}

pub trait Page: AssetQueryable {
    fn name(&self) -> &str;
    fn channel(&self) -> &str;
    fn owner(&self) -> &str;
}

pub trait PageSource: Sized {
    async fn page_at(
        &self,
        owner: &str,
        name: &str,
        channel: &str,
    ) -> Result<impl Page, PageError>;
    async fn pages(&self) -> Result<impl Iterator<Item = impl Page>, PageError>;
}
