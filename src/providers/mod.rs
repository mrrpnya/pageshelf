use forgejo::ForgejoProvider;

use crate::page::{Page, PageError, PageSource};

pub mod forgejo;
pub mod memory;

pub enum ProviderType {
    Forgejo(ForgejoProvider),
}

impl PageSource for ProviderType {
    async fn page_at(
        &self,
        owner: &str,
        name: &str,
        channel: &str,
    ) -> Result<impl Page, PageError> {
        match self {
            Self::Forgejo(v) => v.page_at(owner, name, channel).await,
        }
    }

    async fn pages(&self) -> Result<impl Iterator<Item = impl Page>, PageError> {
        match self {
            Self::Forgejo(v) => v.pages().await,
        }
    }
}
