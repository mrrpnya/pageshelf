use actix_web::{
    middleware::NormalizePath, web::{self, ServiceConfig}, App
};
use minijinja::Environment;

use crate::{conf::ServerConfig, page::PageSource};

pub mod pages;
pub mod server;

pub struct RouteSharedData<'a, PS: PageSource> {
    pub provider: PS,
    pub config: ServerConfig,
    pub jinja: Environment<'a>,
}

pub fn register_to_service_config<'a, PS: PageSource + 'static>(
    config: &'a mut ServiceConfig,
) -> &'a mut ServiceConfig {
    config
        .route(
            "/{owner}/{repo}:{branch}/{file}",
            web::get().to(pages::get_page_orbf::<PS>),
        )
        .route(
            "/{owner}/{repo}:branch",
            web::get().to(pages::get_page_orb::<PS>),
        )
        .route("/{owner}/{repo}", web::get().to(pages::get_page_or::<PS>))
        .route(
            "/{owner}/{repo}/{file}",
            web::get().to(pages::get_page_orf::<PS>),
        )
        .route("/", web::get().to(server::get_index::<PS>))
        .service(server::get_favicon_svg)
}
