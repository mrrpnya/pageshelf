use std::path::Path;

use actix_web::{HttpRequest, HttpResponse, Responder, get, http::StatusCode, web};
use log::{debug, info};
use minijinja::context;
use url::form_urlencoded::Target;

use crate::{
    asset::Asset,
    page::{Page, PageSource},
    resolver::UrlResolution,
    routes::{RouteSharedData, pages::get_page, try_get_page_from_analysis},
    templates::{TEMPLATE_404, TEMPLATE_INDEX, TemplateErrorContext, TemplatePageContext},
};

// TODO: Split the logic for finding a page into its own function
pub async fn get_index<'a, PS: PageSource>(
    data: web::Data<RouteSharedData<'a, PS>>,
    req: HttpRequest,
) -> impl Responder {
    debug!("Index requested");
    let resolution = data.resolver.resolve_http_request(&req);
    match resolution {
        UrlResolution::BuiltIn => {
            info!("Built-In");
            return HttpResponse::Ok().body(
                data.jinja
                    .get_template(TEMPLATE_INDEX)
                    .unwrap()
                    .render(context! {
                        server => data.config.template_server_context()
                    })
                    .unwrap(),
            );
        }
        UrlResolution::Page(loc) => {
            info!("Page: {:?}", loc);
            return get_page(
                &data,
                Some(&loc.page.owner),
                Some(&loc.page.name),
                Some(&loc.page.branch),
                Path::new(&loc.asset),
            )
            .await;
        }
        UrlResolution::External(url) => {
            info!("External URL: {}", url);
            let domains = [url.host_str().unwrap()];
            match data.provider.find_by_domains(&domains).await {
                Ok(page) => {
                    let s = req.uri().to_string();
                    let file = Path::new(&s);
                    return get_page(
                        &data,
                        Some(page.owner()),
                        Some(page.name()),
                        Some(page.branch()),
                        &file,
                    )
                    .await;
                }
                Err(e) => {
                    info!("Failed to find repo by domain \"{}\": {}", url, e);
                }
            }
        }
        _ => {
            return HttpResponse::NotFound().into();
        }
    };
    let tp = data.jinja.get_template(TEMPLATE_404).unwrap();
    return HttpResponse::NotFound().body(
        tp.render(context! {
            server => data.config.template_server_context(),
            page => TemplatePageContext {
                owner: "".to_string(),
                repo: "".to_string()
            },
            error => TemplateErrorContext {
                code: 404,
                message: "Malformed query".to_string()
            }
        })
        .unwrap(),
    );
}

#[get("/pages_favicon.svg")]
async fn get_favicon_svg() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(include_str!("../../branding/pageshelf_logo.svg"))
}
