use actix_web::{HttpResponse, Responder, get, http::StatusCode, web};
use minijinja::context;

use crate::{
    routes::RouteSharedData,
    templates::{TEMPLATE_INDEX, template_server_context},
};

#[get("/")]
async fn get_index<'a>(data: web::Data<RouteSharedData<'a>>) -> impl Responder {
    HttpResponse::with_body(
        StatusCode::OK,
        data.jinja
            .get_template(TEMPLATE_INDEX)
            .unwrap()
            .render(context! {
                server => template_server_context()
            })
            .unwrap(),
    )
}
