//! Upstream managers module
//!
//! Managers help with implementing upstreams by employing high-level actions.

mod cached;
pub use cached::CachedSource;
mod polled;
pub use polled::PolledSource;
