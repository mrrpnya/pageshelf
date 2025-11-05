//! Primary route for accessing Pages (and built-in pages).

use std::time::Instant;

use actix_web::{HttpRequest, HttpResponse, body::MessageBody, http::header::HeaderValue, web};
use tracing::{Level, info, span};

use crate::{frontend::Frontend, server::actix::routes::RoutingState};

pub async fn request_page<F: Frontend>(
    data: web::Data<RoutingState<F>>,
    req: HttpRequest,
) -> HttpResponse {
    let span = span!(
        Level::INFO,
        "request_page",
        client.addr = req.peer_addr().map(|f| f.to_string())
    );
    let _enter = span.enter();
    info!(
        "Requested by {}",
        req.headers()
            .get("Origin")
            .unwrap_or(&HeaderValue::from_str("Unknown Origin").unwrap())
            .to_str()
            .unwrap_or("Unknown Origin")
    );

    let now: Instant = Instant::now();

    let response: HttpResponse = data.frontend.request_url(&req.full_url()).await;

    info!(
        "[{}us] Responding with code {:?}, type: {:?} size: {:?}",
        now.elapsed().as_micros(),
        response.status(),
        response
            .headers()
            .get("Content-Type")
            .map_or("unknown", |f| f.to_str().unwrap_or("error")),
        response.body().size(),
    );

    response
}
