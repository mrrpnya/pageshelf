use actix_web::web::{self, ServiceConfig};
use minijinja::Environment;
use routes::{RoutingState, register_routes_to_config};
use templates::templates_from_builtin;

use crate::{conf::ServerConfig, page::PageSourceFactory, resolver::UrlResolver};

pub mod routes;
pub mod templates;

pub fn setup_service_config<'a, PS: PageSourceFactory + Sync + Send + 'static>(
    web_config: &'a mut ServiceConfig,
    server_config: &'a ServerConfig,
    page_factory: PS,
    templates: Option<Environment<'static>>,
) -> &'a mut ServiceConfig {
    let _pages = server_config.upstream.branches.clone();
    let config = server_config.clone();
    web_config.app_data(web::Data::new(RoutingState {
        provider: page_factory.build().unwrap(),
        jinja: match templates {
            Some(v) => v.clone(),
            None => templates_from_builtin(),
        },
        config,
        resolver: UrlResolver::new(
            server_config.url.clone(),
            server_config.pages_urls.clone(),
            "pages".to_string(),
            "pages".to_string(),
            server_config.allow_domains,
        ),
    }));
    //.wrap(middleware::NormalizePath::trim())
    web_config.configure(|f| {
        register_routes_to_config::<PS::Source>(f);
    });

    web_config
}
