use std::sync::Arc;

use actix_web::web::{self, ServiceConfig};
use minijinja::Environment;
use routes::{RoutingState, register_routes_to_config};
use templates::templates_from_builtin;

use crate::{PageSource, conf::ServerConfig, resolver::UrlResolver};

pub mod routes;
pub mod templates;

pub fn setup_service_config<
    'a,
    PS: PageSource + Sync + Send + 'static,
    UR: UrlResolver + 'static,
>(
    web_config: &'a mut ServiceConfig,
    server_config: &'a ServerConfig,
    page_source: Arc<PS>,
    resolver: UR,
    templates: Option<Environment<'static>>,
) -> &'a mut ServiceConfig {
    let _pages = server_config.upstream.branches.clone();
    let config = server_config.clone();
    web_config.app_data(web::Data::new(RoutingState {
        provider: page_source,
        jinja: match templates {
            Some(v) => v.clone(),
            None => templates_from_builtin(),
        },
        config,
        resolver,
    }));
    //.wrap(middleware::NormalizePath::trim())
    web_config.configure(|f| {
        register_routes_to_config::<PS, UR>(f);
    });

    web_config
}
