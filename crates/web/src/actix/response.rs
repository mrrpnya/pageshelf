use actix_web::HttpResponse;
use pageshelf_frontend::response::{FrontendResponse, FrontendResponseBuilder};

/// Adapter that wraps an Actix `HttpResponse` to implement the
/// `pageshelf_frontend::response::FrontendResponse` trait.
pub struct ActixFrontendResponse(pub HttpResponse);

impl FrontendResponse for ActixFrontendResponse {
    /// The builder type used to construct an Actix response.
    type Builder = ActixFrontendResponseBuilder;

    /// Start building a response with the given HTTP status code.
    fn status(code: u16) -> Self::Builder {
        let status = actix_web::http::StatusCode::from_u16(code)
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
        ActixFrontendResponseBuilder {
            response_builder: HttpResponse::build(status),
        }
    }

    /// Return the response body as a byte slice.
    ///
    /// Note: this adapter stores the body inside Actix's response type and
    /// returns an empty slice here because Actix owns the body data. Callers
    /// should generally inspect the wrapped `HttpResponse` directly.
    fn body(&self) -> &[u8] {
        &[]
    }
}

/// Builder type for constructing an Actix HTTP response.
pub struct ActixFrontendResponseBuilder {
    response_builder: actix_web::HttpResponseBuilder,
}

impl FrontendResponseBuilder for ActixFrontendResponseBuilder {
    type Response = ActixFrontendResponse;

    fn header(mut self, header: &str, value: &str) -> Self {
        self.response_builder.insert_header((header, value));
        self
    }

    fn build(mut self, body: &[u8]) -> Self::Response {
        ActixFrontendResponse(self.response_builder.body(body.to_vec()))
    }
}
