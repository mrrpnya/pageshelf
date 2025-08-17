use serde::{Deserialize, Serialize};

use crate::templates::TemplateServerContext;

fn default_port() -> u16 {
    8080
}

fn default_upstream_url() -> String {
    "https://codeberg.org".to_string()
}

fn default_branch() -> String {
    "pages".to_string()
}

fn default_general() -> ServerConfigGeneral {
    ServerConfigGeneral { 
        name: "Pageshelf".to_string(), 
        home_url: None, 
        port: default_port()
    }
}

fn default_security() -> ServerConfigSecurity {
    ServerConfigSecurity {
        whitelist: None,
        blacklist: None
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfigGeneral {
    name: String,
    home_url: Option<String>,
    #[serde(default = "default_port")]
    port: u16
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerConfigUpstreamType {
    #[serde(rename = "forgejo")]
    Forgejo
}

impl Default for ServerConfigUpstreamType {
    fn default() -> Self {
        Self::Forgejo
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerConfigUpstreamMethod {
    #[serde(rename = "direct")]
    Direct
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
    pub token: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ServerConfigSecurity {
    pub whitelist: Option<String>,
    pub blacklist: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_general")]
    pub general: ServerConfigGeneral,
    #[serde(default = "default_security")]
    pub security: ServerConfigSecurity,
    pub upstream: ServerConfigUpstream
}

impl ServerConfig {
    pub fn template_server_context(&self) -> TemplateServerContext {
        TemplateServerContext {
            name: self.general.name.to_string(),
            about: self.description.to_string(),
            home_url: None,
            icon_url: Some("/favicon.svg".to_string()),
        }
    }
}