use actix_web::{HttpResponse, Responder, get, http::StatusCode, web};
use minijinja::context;

use crate::{routes::RouteSharedData, templates::TEMPLATE_INDEX};

#[get("/")]
async fn get_index<'a>(data: web::Data<RouteSharedData<'a>>) -> impl Responder {
    HttpResponse::with_body(
        StatusCode::OK,
        data.jinja
            .get_template(TEMPLATE_INDEX)
            .unwrap()
            .render(context! {
                server => data.config.template_server_context()
            })
            .unwrap(),
    )
}

#[get("/favicon.svg")]
async fn get_favicon_svg() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(include_str!("../../branding/pageshelf_logo.svg"))
}
