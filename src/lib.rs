pub mod cache;
pub mod asset;

pub trait Page {
    fn name(&self) -> &str;
    fn owner(&self) -> &str;
}