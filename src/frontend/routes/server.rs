/// Primary route for accessing Pages (and built-in pages).
use std::path::Path;

use actix_web::{HttpRequest, HttpResponse, Responder, get, http::header::HeaderValue, web};
use log::{debug, info};
use minijinja::context;

use crate::{
    frontend::{
        routes::{RoutingState, pages::get_page_response},
        templates::{TEMPLATE_ERROR, TEMPLATE_INDEX, TemplateErrorContext, TemplatePageContext},
    },
    page::{Page, PageSource},
    resolver::UrlResolution,
};

pub async fn get_index<'a, PS: PageSource>(
    data: web::Data<RoutingState<'a, PS>>,
    req: HttpRequest,
) -> impl Responder {
    debug!(
        "Requested by {}",
        req.headers()
            .get("Origin")
            .unwrap_or(&HeaderValue::from_str("Unknown Origin").unwrap())
            .to_str()
            .unwrap_or("Unknown Origin")
    );
    let resolution = data.resolver.resolve_http_request(&req);
    match resolution {
        UrlResolution::BuiltIn => {
            info!("Serving Built-In page");
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
            return get_page_response(
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
                    return get_page_response(
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
        _ => {}
    };
    let tp = data.jinja.get_template(TEMPLATE_ERROR).unwrap();
    return HttpResponse::NotFound().body(
        tp.render(context! {
            server => data.config.template_server_context(),
            page => TemplatePageContext {
                owner: "".to_string(),
                repo: "".to_string()
            },
            error => TemplateErrorContext {
                code: 404,
                message: "Malformed query".to_string(),
                about: "Failed to analyze query.".to_string()
            }
        })
        .unwrap(),
    );
}

#[get("/pages_favicon.png")]
pub async fn get_favicon_png() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/png")
        .body(std::include_bytes!("../../../branding/pageshelf_logo.png").as_slice())
}
