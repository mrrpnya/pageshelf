use crate::asset::AssetQueryable;

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

pub trait PageSource {
    async fn page_at(&self, owner: &str, name: &str, channel: &str)
    -> Result<impl Page, PageError>;
    async fn pages(&self) -> Result<impl Iterator<Item = impl Page>, PageError>;
}
