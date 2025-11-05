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

pub trait FrontendResponseBuilder {
    type Response: FrontendResponse;

    fn content_type(self, mime_type: &str) -> Self;
    /// Sets the `Location` header for redirection responses.
    fn with_location(self, location: &str) -> Self;

    /// Finalizes and builds the response.
    fn body(self, body: &[u8]) -> Self::Response;
}

/* -------------------------------------------------------------------------- */
/*                 Default implementation of FrontendResponse                 */
/* -------------------------------------------------------------------------- */

pub struct DefaultFrontendResponse {
    pub status_code: u16,
    pub body: Vec<u8>,
    pub mime_type: Option<String>,
}

impl FrontendResponse for DefaultFrontendResponse {
    type Builder = DefaultFrontendResponseBuilder;

    fn status(status_code: u16) -> Self::Builder {
        DefaultFrontendResponseBuilder {
            status_code,
            location: None,
            body: Vec::new(),
            mime_type: None,
        }
    }

    fn body(&self) -> &[u8] {
        self.body.as_slice()
    }
}

pub struct DefaultFrontendResponseBuilder {
    pub status_code: u16,
    pub location: Option<String>,
    pub body: Vec<u8>,
    pub mime_type: Option<String>,
}

impl FrontendResponseBuilder for DefaultFrontendResponseBuilder {
    type Response = DefaultFrontendResponse;

    fn content_type(mut self, mime_type: &str) -> Self {
        self.mime_type = Some(mime_type.to_string());
        self
    }

    fn with_location(mut self, location: &str) -> Self {
        self.location = Some(location.to_string());
        self
    }

    fn body(self, body: &[u8]) -> Self::Response {
        DefaultFrontendResponse {
            status_code: self.status_code,
            body: body.to_vec(),
            mime_type: self.mime_type,
        }
    }
}
