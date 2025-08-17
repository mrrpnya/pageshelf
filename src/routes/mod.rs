use minijinja::Environment;

use crate::{conf::ServerConfig, providers::forgejo::ForgejoProvider, PageSource};

pub mod pages;
pub mod server;

pub enum UpstreamProviderType {
    Forgejo(ForgejoProvider),
}

impl PageSource for UpstreamProviderType {
    async fn page_at(
        &self,
        owner: &str,
        name: &str,
        channel: &str,
    ) -> Result<impl crate::Page, crate::PageError> {
        match self {
            Self::Forgejo(v) => v.page_at(owner, name, channel).await,
        }
    }

    async fn pages(&self) -> Result<impl Iterator<Item = impl crate::Page>, crate::PageError> {
        match self {
            Self::Forgejo(v) => v.pages().await,
        }
    }
}

pub struct RouteSharedData<'a> {
    pub provider: UpstreamProviderType,
    pub config: ServerConfig,
    pub jinja: Environment<'a>,
}
