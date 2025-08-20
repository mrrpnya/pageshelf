use clap::crate_version;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::templates::TemplateServerContext;

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
    }
}

fn default_user() -> String {
    "admin".to_string()
}

fn default_redis() -> ServerConfigRedis {
    ServerConfigRedis {
        enabled: default_redis_enabled(),
        address: default_redis_address(),
        port: default_redis_port(),
    }
}

fn default_redis_enabled() -> bool {
    false
}

fn default_redis_address() -> String {
    "127.0.0.1".to_string()
}

fn default_redis_port() -> u16 {
    6379
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerConfigUpstreamType {
    #[serde(rename = "forgejo")]
    Forgejo,
}

impl Default for ServerConfigUpstreamType {
    fn default() -> Self {
        Self::Forgejo
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerConfigUpstreamMethod {
    #[serde(rename = "direct")]
    Direct,
}

impl Default for ServerConfigUpstreamMethod {
    fn default() -> Self {
        Self::Direct
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfigUpstream {
    #[serde(default)]
    pub r#type: ServerConfigUpstreamType,
    #[serde(default)]
    pub method: ServerConfigUpstreamMethod,
    #[serde(default = "default_upstream_url")]
    pub url: String,
    #[serde(default = "default_branch")]
    pub default_branch: String,
    #[serde(default = "default_branches_allowed")]
    pub branches: Vec<String>,
    pub token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfigSecurity {
    pub whitelist: Option<String>,
    pub blacklist: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfigRedis {
    #[serde(default = "default_redis_enabled")]
    pub enabled: bool,
    #[serde(default = "default_redis_address")]
    pub address: String,
    #[serde(default = "default_redis_port")]
    pub port: u16,
}

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

    // Specialized
    #[serde(default = "default_security")]
    pub security: ServerConfigSecurity,
    pub upstream: ServerConfigUpstream,
    #[serde(default = "default_redis")]
    pub redis: ServerConfigRedis,
}

impl ServerConfig {
    pub fn template_server_context(&self) -> TemplateServerContext {
        TemplateServerContext {
            name: self.name.to_string(),
            about: self.description.to_string(),
            home_url: None,
            icon_url: Some("/pages_favicon.svg".to_string()),
            default_branch: self.upstream.default_branch.clone(),
            version: crate_version!(),
        }
    }
}

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

            // Specialized
            security: ServerConfigSecurity {
                whitelist: None,
                blacklist: None,
            },
            upstream: ServerConfigUpstream {
                r#type: ServerConfigUpstreamType::Forgejo,
                method: ServerConfigUpstreamMethod::Direct,
                url: "".to_string(),
                default_branch: default_branch(),
                branches: Vec::new(),
                token: None,
            },
            redis: default_redis(),
        }
    }
}
