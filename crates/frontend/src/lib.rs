#![forbid(unsafe_code)]
//#![warn(missing_docs)]
use url::Url;

mod default;
use crate::response::FrontendResponse;

pub mod headers;
pub mod response;

pub use default::DefaultFrontend;
pub mod renderer;

#[allow(async_fn_in_trait)]
pub trait Frontend {
    async fn request_url<R: FrontendResponse>(&self, url: &Url) -> R;
}
