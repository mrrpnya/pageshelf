use std::{path::Path, str::FromStr};

use actix_web::{HttpRequest, HttpResponse, Responder, get, http::StatusCode, web};
use minijinja::context;
use url::Url;

use crate::{
    page::PageSource,
    routes::{
        RouteSharedData,
        pages::{get_page, is_base_url},
        try_get_page_from_analysis,
    },
    templates::{TEMPLATE_404, TEMPLATE_INDEX, TemplateErrorContext, TemplatePageContext},
    util::{UrlAnalysis, analyze_url},
};

pub async fn get_index<'a, PS: PageSource>(
    data: web::Data<RouteSharedData<'a, PS>>,
    req: HttpRequest,
) -> impl Responder {
    if is_base_url(&data, &req) {
        return HttpResponse::build(StatusCode::OK).body(
            data.jinja
                .get_template(TEMPLATE_INDEX)
                .unwrap()
                .render(context! {
                    server => data.config.template_server_context()
                })
                .unwrap(),
        );
    } else {
        if let Some(page) = try_get_page_from_analysis(&data, &req).await {
            return page;
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

#[get("/favicon.svg")]
async fn get_favicon_svg() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(include_str!("../../branding/pageshelf_logo.svg"))
}
