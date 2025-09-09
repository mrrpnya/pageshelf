/// Caches allow for fast, local storage of page data.

#[derive(Debug, PartialEq, Eq)]
pub enum CacheError {
    ConnectionError,
    OperationError(String),
    NotFound,
}

pub trait CacheConnection {
    #[allow(async_fn_in_trait)]
    async fn set(&mut self, key: &str, value: &str) -> Result<(), CacheError>;
    #[allow(async_fn_in_trait)]
    async fn get(&mut self, key: &str) -> Result<String, CacheError>;
    #[allow(async_fn_in_trait)]
    async fn delete(&mut self, key: &str) -> Result<u32, CacheError>;
}

pub trait Cache: Clone {
    type Connection: CacheConnection;
    #[allow(async_fn_in_trait)]
    async fn connect(&self) -> Result<Self::Connection, CacheError>;
}
