//! Primary route for accessing Pages (and built-in pages).

use std::time::Instant;

use actix_web::{HttpRequest, HttpResponse, body::MessageBody, http::header::HeaderValue, web};
use pageshelf_frontend::Frontend;
use tracing::{Level, info, span};

use crate::actix::{response::ActixFrontendResponse, routes::RoutingState};

/// Actix handler that delegates URL resolution to the configured `Frontend`.
///
/// This handler is the single catch-all entry point used by Pageshelf.
/// It performs request logging and
/// delegates the actual URL resolution to the `Frontend` implementation
/// via `request_url`, and converts the returned `FrontendResponse` into
/// an Actix `HttpResponse`.
///
/// Notes:
/// - The `Frontend` implementation is responsible for producing the
///   correct HTTP status, headers and body. This function merely
///   forwards that response to the client and records timing info.
pub async fn main_route<F: Frontend>(
    data: web::Data<RoutingState<F>>,
    req: HttpRequest,
) -> HttpResponse {
    let span = span!(
        Level::INFO,
        "main_route",
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

    let response = data
        .frontend
        .request_url::<ActixFrontendResponse>(&req.full_url())
        .await;

    info!(
        "[{}us] Responding with code {:?}, type: {:?} size: {:?}",
        now.elapsed().as_micros(),
        response.0.status(),
        response
            .0
            .headers()
            .get("Content-Type")
            .map_or("unknown", |f| f.to_str().unwrap_or("error")),
        response.0.body().size(),
    );

    response.0
}
