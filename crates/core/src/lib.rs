//! Pageshelf: Core
//!
//! This provides a common interface and structure for the Pageshelf service.
//!
//! ## License
//!
//! Licensed under the terms of the MIT License.
#![warn(missing_docs)]
// TODO: Allow for data transformation when obtained through the upstream
// TODO: Find more resources on proper code layout
#[forbid(unsafe_code)]
//#[deprecated]
//mod asset;
//#[deprecated]
//pub mod resolver;
//pub use asset::*;
pub mod cache;
pub mod domain;
pub mod event;
pub mod ext;
pub mod resolution;
pub mod upstream;
//mod util;
