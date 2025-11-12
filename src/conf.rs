//! Configuration schema and utilities for Pageshelf.

use color_eyre::{
    Section,
    eyre::{self, Context},
};
use config::Config;
use serde::{Deserialize, Serialize};
use url::Url;

// TODO: Should all configuration be done in this master file?
// ? It seems like a potentially better idea to move configuration to where it's needed.

/* -------------------------------------------------------------------------- */
/*                              Config structure                              */
/* -------------------------------------------------------------------------- */

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ServerConfigUpstreamType {
    #[serde(rename = "forgejo")]
    #[default]
    Forgejo,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ServerCacheBackend {
    #[serde(rename = "redis")]
    #[default]
    Redis,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ServerConfigUpstreamMethod {
    #[serde(rename = "direct")]
    #[default]
    Direct,
}

/// Upstream configuration for the server.
/// This configures where to get page data from.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfigUpstream {
    /// What type of platform
    #[serde(default)]
    pub r#type: ServerConfigUpstreamType,
    /// How to get data from that platform
    #[serde(default)]
    pub method: ServerConfigUpstreamMethod,
    pub url: Url,
    #[serde(default = "default_branch")]
    pub default_branch: String,
    #[serde(default = "default_branches_allowed")]
    pub branches: Vec<String>,
    pub token: Option<String>,
    pub poll_interval: Option<u64>,
}

/// Cache configuration for the server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfigCache {
    pub backend: ServerCacheBackend,
    /// Should Cache be used?
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,
    /// Where to find the Cache server (address)
    #[serde(default = "default_cache_address")]
    pub address: String,
    /// Where to find the Cache server (port)
    #[serde(default = "default_cache_port")]
    pub port: u16,
    /// How long should cached assets live in Cache?
    #[serde(default = "default_cache_ttl")]
    pub ttl: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfigMetrics {
    pub enabled: bool,
    pub port: u16,
}

/// Aggregate configuration of the server (Contains all other configs)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfig {
    // General
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default = "default_description")]
    pub description: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub domain: Option<Url>,
    pub pages_domains: Option<Vec<Url>>,
    #[serde(default = "default_domains_allowed")]
    pub allow_domains: bool,

    // Specialized
    pub upstream: ServerConfigUpstream,
    #[serde(default = "default_cache")]
    pub cache: ServerConfigCache,
    #[serde(default = "default_metrics_config")]
    pub metrics: ServerConfigMetrics,
}

impl ServerConfig {
    pub fn from_config(config: &Config) -> Result<Self, eyre::Report> {
        config
            .clone()
            .try_deserialize::<ServerConfig>()
            .wrap_err("Failed to deserialize configuration")
            .suggestion("Check your configuration file and/or environment variables")
    }
}

/* ---------------------------------- Serde --------------------------------- */

/* -------------------------------------------------------------------------- */
/*                            Default initializers                            */
/* -------------------------------------------------------------------------- */

fn default_port() -> u16 {
    8080
}

fn default_name() -> String {
    "Pageshelf".to_string()
}

fn default_description() -> String {
    "A free and open source Pages server, written in Rust".to_string()
}

fn default_branch() -> String {
    "pages".to_string()
}

fn default_branches_allowed() -> Vec<String> {
    vec!["pages".to_string()]
}

fn default_cache() -> ServerConfigCache {
    ServerConfigCache {
        backend: ServerCacheBackend::Redis,
        enabled: default_cache_enabled(),
        address: default_cache_address(),
        port: default_cache_port(),
        ttl: default_cache_ttl(),
    }
}

fn default_cache_enabled() -> bool {
    false
}

fn default_cache_address() -> String {
    "127.0.0.1".to_string()
}

fn default_cache_port() -> u16 {
    6379
}

fn default_cache_ttl() -> Option<u32> {
    None
}

fn default_domains_allowed() -> bool {
    false
}

fn default_metrics_config() -> ServerConfigMetrics {
    ServerConfigMetrics {
        enabled: false,
        port: 9000,
    }
}
