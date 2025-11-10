//! Web utilities and server implementations for Pageshelf.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use color_eyre::eyre;
use pageshelf_frontend::Frontend;

#[cfg(feature = "actix")]
pub mod actix;

/// High-level abstraction for launching a web server that serves a Frontend.
///
/// Allows the application (and tests)
/// to start different server implementations interchangeably (for
/// example an Actix-based server or a test harness). Implementations
/// encapsulate binding, middleware and route setup; callers simply
/// request that the server start on a host and port. Errors should be
/// returned as an `eyre::Report` so callers receive rich diagnostic
/// context across async boundaries.
#[allow(async_fn_in_trait)]
pub trait WebServer {
    /// The frontend implementation served by this web server.
    type Frontend: Frontend;

    /// Start the server on the given host and port.
    async fn run(&self, host: &str, port: u16) -> Result<(), eyre::Report>;
}
