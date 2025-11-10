use serde::{Deserialize, Serialize};
use url::Url;

/// The configuration needed to create a [ForgejoUpstream](super::ForgejoUpstream).
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ForgejoUpstreamConfig {
    /// The reachable location of a valid Forgejo instance.
    pub url: Url,
    /// Contains branches of a repository on the instance that are allowed to be used as pages.
    pub branches: Vec<String>,
    /// How long, in seconds, to wait after a scan before scanning again.
    ///
    /// Lower values can lead to faster refresh times, but more load.
    /// Will default to every 4 minutes if not specified.
    pub scan_interval: Option<u64>,
}
