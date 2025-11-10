use std::collections::HashMap;

/* -------------------------------------------------------------------------- */
/*                                  Response                                  */
/* -------------------------------------------------------------------------- */

/// A trait representing a frontend response, or way of constructing one.
///
/// Recommended to implement this on a per-framework basis, e.g., for Actix-web, etc.
pub trait FrontendResponse {
    type Builder: FrontendResponseBuilder<Response = Self>;

    /// Creates a FrontendResponseBuilder for the given status code, which can be used to further customize the response.
    ///
    /// See https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status for standard status codes.
    fn status(status_code: u16) -> Self::Builder;

    /* ---------------------- Predefined response statuses ---------------------- */

    #[inline]
    fn ok() -> Self::Builder {
        Self::status(200)
    }
    #[inline]
    fn not_found() -> Self::Builder {
        Self::status(404)
    }
    #[inline]
    fn internal_server_error() -> Self::Builder {
        Self::status(500)
    }
    #[inline]
    fn im_a_teapot() -> Self::Builder {
        Self::status(418)
    }
    #[inline]
    fn unauthorized() -> Self::Builder {
        Self::status(401)
    }
    #[inline]
    fn no_content() -> Self::Builder {
        Self::status(204)
    }
    #[inline]
    fn forbidden() -> Self::Builder {
        Self::status(403)
    }
    #[inline]
    fn gone() -> Self::Builder {
        Self::status(410)
    }
    #[inline]
    fn too_many_requests() -> Self::Builder {
        Self::status(429)
    }

    fn body(&self) -> &[u8];
}

pub trait FrontendResponseBuilder: Sized {
    type Response: FrontendResponse;

    /* --------------------------------- Headers -------------------------------- */

    fn header(self, header: &str, value: &str) -> Self;

    /// Sets the `Content-Type` header.
    fn content_type(self, mime_type: &str) -> Self {
        self.header("Content-Type", mime_type)
    }

    /// Sets the `Location` header (commonly used for redirects).
    fn location(self, location: &str) -> Self {
        self.header("Location", location)
    }

    /* -------------------------- Quick response types -------------------------- */

    /// Shortcut for setting plain text response headers.
    fn text(self, body: &str) -> Self::Response {
        self.content_type("text/plain; charset=utf-8")
            .build(body.as_bytes())
    }

    /// Shortcut for setting HTML response headers.
    fn html(self, body: &str) -> Self::Response {
        self.content_type("text/html; charset=utf-8")
            .build(body.as_bytes())
    }

    fn octet_stream(self, body: &[u8]) -> Self::Response {
        self.content_type("application/octet-stream").build(body)
    }

    /// Shortcut for redirect responses (adds Location header and sets a body).
    fn redirect(self, location: &str) -> Self::Response {
        self.header("Location", location)
            .build(format!("Redirecting to {location}").as_bytes())
    }

    /* ------------------------------ Finalization ------------------------------ */

    /// Adds a body, finalizing the response.
    fn build(self, body: &[u8]) -> Self::Response;
}

/* -------------------------------------------------------------------------- */
/*                                    Mock                                    */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, PartialEq)]
pub struct MockFrontendResponse {
    status_code: u16,
    body: Vec<u8>,
    headers: HashMap<String, String>,
}

impl FrontendResponse for MockFrontendResponse {
    type Builder = DefaultFrontendResponseBuilder;

    fn status(status_code: u16) -> Self::Builder {
        DefaultFrontendResponseBuilder {
            status_code,
            headers: HashMap::new(),
        }
    }

    fn body(&self) -> &[u8] {
        self.body.as_slice()
    }
}

pub struct DefaultFrontendResponseBuilder {
    status_code: u16,
    headers: HashMap<String, String>,
}

impl FrontendResponseBuilder for DefaultFrontendResponseBuilder {
    type Response = MockFrontendResponse;

    fn header(mut self, header: &str, value: &str) -> Self {
        self.headers.insert(header.to_string(), value.to_string());
        self
    }

    fn build(self, body: &[u8]) -> Self::Response {
        MockFrontendResponse {
            status_code: self.status_code,
            body: body.to_vec(),
            headers: self.headers,
        }
    }
}
