use std::path::Path;

use actix_web::{
    HttpRequest, HttpResponse, Responder, get, http::StatusCode, web,
};
use log::{debug, info};
use minijinja::context;

use crate::{
    asset::Asset,
    page::{Page, PageSource},
    routes::{
        RouteSharedData,
        pages::{get_page, is_base_url, is_page_url},
        try_get_page_from_analysis,
    },
    templates::{TEMPLATE_404, TEMPLATE_INDEX, TemplateErrorContext, TemplatePageContext},
};

// TODO: Split the logic for finding a page into its own function
pub async fn get_index<'a, PS: PageSource>(
    data: web::Data<RouteSharedData<'a, PS>>,
    req: HttpRequest,
) -> impl Responder {
    debug!("Index requested");
    if is_base_url(&data, &req) || (data.config.url.is_none() && data.config.pages_urls.is_none()) {
        if req.uri().to_string() == "/" || req.uri().to_string() == "/index" {
            return HttpResponse::build(StatusCode::OK).body(
                data.jinja
                    .get_template(TEMPLATE_INDEX)
                    .unwrap()
                    .render(context! {
                        server => data.config.template_server_context()
                    })
                    .unwrap(),
            );
        }

        if let Some(page) = try_get_page_from_analysis(&data, &req).await {
            return page;
        }
    } else {
        if is_page_url(&data, &req) {
            if let Some(page) = try_get_page_from_analysis(&data, &req).await {
                return page;
            }
        } else {
            if let Some(host) = req.headers().get("Host") {
                let host = host.to_str().unwrap();
                info!("Finding repo by domain \"{}\"...", host);
                let d_start = match host.starts_with("www.") {
                    true => host.find('.').unwrap() + 1,
                    false => 0,
                };
                let d_end = match host.contains(":") {
                    true => host.find(':').unwrap(),
                    false => host.len(),
                };
                let domains = [&host[0..d_end], &host[d_start..d_end]];
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
                        info!("Failed to find repo by domain \"{}\": {}", host, e);
                    }
                }
            }
        }
    }

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
