use actix_web::HttpResponse;

use crate::frontend::response::{FrontendResponse, FrontendResponseBuilder};

impl FrontendResponse for HttpResponse {
    type Builder = ActixFrontendResponseBuilder;

    fn status(code: u16) -> Self::Builder {
        let status = actix_web::http::StatusCode::from_u16(code)
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
        ActixFrontendResponseBuilder {
            response_builder: HttpResponse::build(status),
        }
    }

    fn body(&self) -> &[u8] {
        &[]
    }
}

pub struct ActixFrontendResponseBuilder {
    response_builder: actix_web::HttpResponseBuilder,
}

impl FrontendResponseBuilder for ActixFrontendResponseBuilder {
    type Response = HttpResponse;

    fn content_type(mut self, mime_type: &str) -> Self {
        self.response_builder.content_type(mime_type);
        self
    }

    fn with_location(mut self, location: &str) -> Self {
        self.response_builder.insert_header((
            actix_web::http::header::LOCATION,
            actix_web::http::header::HeaderValue::from_str(location).unwrap(),
        ));
        self
    }

    fn body(mut self, body: &[u8]) -> Self::Response {
        self.response_builder.body(body.to_vec())
    }
}
