//! Configuration schema and utilities for Pageshelf.

use clap::crate_version;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{frontend::templates::TemplateServerContext, resolver::DefaultUrlResolver};

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
    #[serde(default = "default_upstream_url")]
    pub url: String,
    #[serde(default = "default_branch")]
    pub default_branch: String,
    #[serde(default = "default_branches_allowed")]
    pub branches: Vec<String>,
    pub token: Option<String>,
    pub poll_interval: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfigSecurity {
    pub whitelist: Option<String>,
    pub blacklist: Option<String>,
    #[serde(default = "default_security_show_private")]
    pub show_private: bool,
}

/// Cache configuration for the server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfigCache {
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
    pub url: Option<Url>,
    pub pages_urls: Option<Vec<Url>>,
    #[serde(default = "default_user")]
    pub default_user: String,
    #[serde(default = "default_domains_allowed")]
    pub allow_domains: bool,

    // Specialized
    #[serde(default = "default_security")]
    pub security: ServerConfigSecurity,
    pub upstream: ServerConfigUpstream,
    #[serde(default = "default_cache")]
    pub cache: ServerConfigCache,
}

impl ServerConfig {
    pub fn template_server_context(&self) -> TemplateServerContext {
        TemplateServerContext {
            name: self.name.to_string(),
            about: self.description.to_string(),
            url: self.url.as_ref().map(|v| v.as_str().to_string()),
            icon_url: Some("/pages_favicon.webp".to_string()),
            default_branch: self.upstream.default_branch.clone(),
            version: crate_version!(),
        }
    }

    pub fn url_resolver(&self) -> DefaultUrlResolver {
        DefaultUrlResolver::new(
            self.url.clone(),
            self.pages_urls.clone(),
            "pages".to_string(),
            "pages".to_string(),
            self.allow_domains,
        )
    }
}

/* ---------------------------------- Serde --------------------------------- */

/* -------------------------------------------------------------------------- */
/*                            Default initializers                            */
/* -------------------------------------------------------------------------- */

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            // General
            name: default_name(),
            description: default_description(),
            url: None,
            pages_urls: None,
            port: default_port(),
            default_user: default_user(),
            allow_domains: default_domains_allowed(),

            // Specialized
            security: ServerConfigSecurity {
                whitelist: None,
                blacklist: None,
                show_private: default_security_show_private(),
            },
            upstream: ServerConfigUpstream {
                r#type: ServerConfigUpstreamType::Forgejo,
                method: ServerConfigUpstreamMethod::Direct,
                poll_interval: None,
                url: "".to_string(),
                default_branch: default_branch(),
                branches: Vec::new(),
                token: None,
            },
            cache: default_cache(),
        }
    }
}

fn default_port() -> u16 {
    8080
}

fn default_name() -> String {
    "Pageshelf".to_string()
}

fn default_description() -> String {
    "A free and open source Pages server, written in Rust".to_string()
}

fn default_upstream_url() -> String {
    "https://codeberg.org".to_string()
}

fn default_branch() -> String {
    "pages".to_string()
}

fn default_branches_allowed() -> Vec<String> {
    vec!["pages".to_string()]
}

fn default_security() -> ServerConfigSecurity {
    ServerConfigSecurity {
        whitelist: None,
        blacklist: None,
        show_private: default_security_show_private(),
    }
}

fn default_security_show_private() -> bool {
    false
}

fn default_user() -> String {
    "admin".to_string()
}

fn default_cache() -> ServerConfigCache {
    ServerConfigCache {
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
