//! Pageshelf is a free and open source Pages server, written in Rust.
#![forbid(unsafe_code)]
use frontend::routes::RoutingState;

mod core;
pub use core::*;

pub mod conf;
pub mod frontend;
pub mod provider;
//pub mod util;
