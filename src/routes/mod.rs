use minijinja::Environment;

use crate::{conf::ServerConfig, providers::ProviderType};

pub mod pages;
pub mod server;

pub struct RouteSharedData<'a> {
    pub provider: ProviderType,
    pub config: ServerConfig,
    pub jinja: Environment<'a>,
}
