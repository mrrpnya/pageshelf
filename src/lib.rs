//! <div align="center">
//!
//! <img src="logo.svg" width="100" alt="Logo"/>
//!
//! # Pageshelf
//!
//! A free and open-source Pages server, written in safe Rust.
//!
//! ![GitHub branch check runs](https://img.shields.io/github/check-runs/mrrpnya/pageshelf/main)
//! ![GitHub License](https://img.shields.io/github/license/mrrpnya/pageshelf)
//! ![Unsafe Forbidden](https://img.shields.io/badge/unsafe-forbidden-success)
//!
//! </div>
#![forbid(unsafe_code)]

pub use pageshelf_core::*;
pub use pageshelf_frontend::*;
pub use pageshelf_web::*;

pub mod conf;

pub mod cache_combiner;
pub mod log;
