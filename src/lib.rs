use actix_web::web::{self, ServiceConfig};
use conf::ServerConfig;
use minijinja::Environment;
use page::PageSourceFactory;
use routes::RouteSharedData;
use templates::templates_from_builtin;

pub mod asset;
pub mod conf;
pub mod page;
pub mod providers;
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
    web_config.app_data(web::Data::new(RouteSharedData {
        provider: page_factory.build().unwrap(),
        jinja: match templates {
            Some(ref v) => v.clone(),
            None => templates_from_builtin(),
        },
        config,
    }));
    //.wrap(middleware::NormalizePath::trim())
    web_config.configure(move |f| {
        routes::setup_service_config::<PS>(f, server_config, page_factory, templates);
    });

    web_config
}
