pub mod forgejo;
pub mod memory;
pub mod layers;

// Export specific types
pub use forgejo::ForgejoProvider;
pub use forgejo::ForgejoProviderFactory;
pub use memory::MemoryPageProvider;
pub use memory::MemoryPageProviderFactory;

pub mod testing {
    pub use super::memory::testing::create_example_provider;
    pub use super::memory::testing::create_example_provider_factory;
    pub use super::memory::testing::test_example_source;
}
