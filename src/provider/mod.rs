pub mod cache;
#[cfg(feature = "forgejo")]
pub mod forgejo;
#[cfg(feature = "gitea")]
pub mod gitea;
#[cfg(feature = "gitlab")]
pub mod gitlab;
pub mod layers;
pub mod memory;
mod scanner;

// Export specific types
#[cfg(feature = "forgejo")]
pub use forgejo::ForgejoProvider;
#[cfg(feature = "forgejo")]
pub use forgejo::ForgejoProviderFactory;
pub use memory::MemoryPageProvider;
pub use memory::MemoryPageProviderFactory;

pub mod testing {
    pub use super::memory::testing::create_example_provider;
    pub use super::memory::testing::create_example_provider_factory;
    pub use super::memory::testing::test_example_source;
}
