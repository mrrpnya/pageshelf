use std::{path::Path, str::FromStr};

use actix_web::{
    HttpRequest, HttpResponse,
    web::{self, ServiceConfig},
};
use log::debug;
use minijinja::Environment;
use pages::get_page;
use url::Url;

use crate::{
    conf::ServerConfig,
    page::{PageSource, PageSourceFactory},
    resolver::{UrlResolution, UrlResolver}
};

pub mod pages;
pub mod server;

/// This serves as state for the Actix server.
/// TODO: Rename?
pub struct RouteSharedData<'a, PS: PageSource> {
    pub provider: PS,
    pub config: ServerConfig,
    pub jinja: Environment<'a>,
    pub resolver: UrlResolver,
}

async fn try_get_page_from_analysis<'a, PS: PageSource>(
    data: &web::Data<RouteSharedData<'a, PS>>,
    req: &HttpRequest,
) -> Option<HttpResponse> {
    let resolution = data.resolver.resolve_http_request(&req);

    match resolution {
        UrlResolution::Page(loc) => {
            return Some(
                get_page(
                    &data,
                    Some(loc.page.owner.as_str()),
                    Some(loc.page.name.as_str()),
                    Some(loc.page.branch.as_str()),
                    Path::new(&loc.asset),
                )
                .await,
            );
        },
        _ => {
            Some(HttpResponse::NotFound().finish())
        }
    }

}

/* -------------------------------------------------------------------------- */
/*                                Registration                                */
/* -------------------------------------------------------------------------- */

/// Register default routes for the server to an Actix configuration.
fn register_routes_to_config<'a, PS: PageSource + 'static>(
    config: &'a mut ServiceConfig,
) -> &'a mut ServiceConfig {
    config
        /*  .route(
            "/{owner}/{repo}:{branch}/{file:.*}",
            web::get().to(pages::get_page_orbf::<PS>),
        )
        .route(
            "/{owner}/{repo}:{branch}",
            web::get().to(pages::get_page_orb::<PS>),
        )
        .route("/{owner}/{repo}", web::get().to(pages::get_page_or::<PS>))
        .route(
            "/{owner}/{repo}/{file:.*}",
            web::get().to(pages::get_page_orf::<PS>),
        )*/
        .service(server::get_favicon_svg)
        .route("/{tail:.*}", web::get().to(server::get_index::<PS>))
}

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
            Some(v) => v.clone(),
            None => crate::templates::templates_from_builtin(),
        },
        config,
        resolver: UrlResolver::new(
            server_config.url.clone(),
            server_config.pages_urls.clone(),
            "pages".to_string(),
            "pages".to_string(),
            server_config.allow_domains
        ),
    }));
    //.wrap(middleware::NormalizePath::trim())
    web_config.configure(|f| {
        register_routes_to_config::<PS::Source>(f);
    });

    web_config
}
