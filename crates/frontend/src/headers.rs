use std::borrow::Cow;

/// Represents an HTTP `Content-Type` header with optional parameters.
///
/// Internally, this stores the full header value as a single string for efficiency.
/// The builder methods (`charset`, `boundary`, `directive`) modify this string,
/// and debug assertions ensure correctness in debug builds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentType {
    value: Cow<'static, str>,
}

impl ContentType {
    /// Creates a new `Content-Type` from a MIME type.
    pub const fn const_new(mime_type: &'static str) -> Self {
        Self {
            value: Cow::Borrowed(mime_type),
        }
    }

    /// Runtime constructor for dynamically provided MIME types.
    ///
    /// # Panics (debug only)
    /// - If the mime_type is determined to be malformed
    pub fn new(mime_type: &str) -> Self {
        #[cfg(debug_assertions)]
        {
            let trimmed = mime_type.trim();
            debug_assert!(
                !trimmed.is_empty() && trimmed.contains('/') && !trimmed.ends_with('/'),
                "Invalid MIME type '{}'",
                mime_type
            );
        }
        Self {
            value: Cow::Owned(mime_type.trim().to_string()),
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    #[inline]
    pub fn to_header_value(&self) -> String {
        self.value.to_string()
    }

    #[inline(always)]
    fn push_directive(&mut self, key: &str, value: &str) {
        if let Cow::Borrowed(s) = &self.value {
            // Upgrade to owned before mutating
            self.value = Cow::Owned(s.to_string());
        }

        let s = self.value.to_mut();
        s.push_str("; ");
        s.push_str(key);
        s.push('=');
        s.push_str(value);
    }

    /// Adds a `charset` directive.
    ///
    /// # Panics (debug only)
    /// - If the charset is empty or contains whitespace.
    pub fn charset(mut self, charset: &str) -> Self {
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                !charset.is_empty(),
                "ContentType::charset(): charset must not be empty"
            );
            debug_assert!(
                !charset.contains(char::is_whitespace),
                "ContentType::charset(): charset '{}' contains whitespace, which is invalid",
                charset
            );
        }

        self.push_directive("charset", charset);
        self
    }

    /// Adds a `boundary` directive (used in multipart types).
    ///
    /// # Panics (debug only)
    /// - If the boundary is empty or contains spaces or quotes.
    pub fn boundary(mut self, boundary: &str) -> Self {
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                !boundary.is_empty(),
                "ContentType::boundary(): boundary must not be empty"
            );
            debug_assert!(
                !boundary.contains(char::is_whitespace),
                "ContentType::boundary(): boundary '{}' contains whitespace, which is invalid",
                boundary
            );
            debug_assert!(
                !boundary.contains('"'),
                "ContentType::boundary(): boundary '{}' contains quotes, which is invalid",
                boundary
            );
        }

        self.push_directive("boundary", boundary);
        self
    }

    /// Adds an arbitrary directive (e.g., `profile`, `version`, etc.).
    ///
    /// # Panics (debug only)
    /// - If the key or value are empty or contain invalid characters.
    pub fn directive(mut self, key: &str, value: &str) -> Self {
        if cfg!(debug_assertions) {
            debug_assert!(
                !key.trim().is_empty(),
                "ContentType::directive(): key must not be empty"
            );
            debug_assert!(
                !value.trim().is_empty(),
                "ContentType::directive(): value must not be empty"
            );
            debug_assert!(
                !key.contains(|c: char| c.is_whitespace() || c == ';'),
                "ContentType::directive(): key '{}' contains invalid characters",
                key
            );
            debug_assert!(
                !value.contains(';'),
                "ContentType::directive(): value '{}' contains ';', which is invalid",
                value
            );
        }

        self.push_directive(key, value);
        self
    }

    /* ----------------------------- Convenience Types ----------------------------- */

    pub const TEXT: Self = Self::const_new("text/plain");
    pub const HTML: Self = Self::const_new("text/html");
    pub const JSON: Self = Self::const_new("application/json");
    pub const OCTET_STREAM: Self = Self::const_new("application/octet-stream");

    #[inline]
    pub fn text() -> Self {
        Self::TEXT.clone()
    }
    #[inline]
    pub fn html() -> Self {
        Self::HTML.clone()
    }
    #[inline]
    pub fn json() -> Self {
        Self::JSON.clone()
    }
    #[inline]
    pub fn octet_stream() -> Self {
        Self::OCTET_STREAM.clone()
    }

    /// `multipart/form-data; boundary=...`
    pub fn multipart(boundary: &str) -> Self {
        Self::new("multipart/form-data").boundary(boundary)
    }
}

impl Default for ContentType {
    fn default() -> Self {
        Self::TEXT.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("text/plain", "text/plain")]
    #[case("application/json", "application/json")]
    fn test_new_content_type(#[case] input: &str, #[case] expected: &str) {
        let ct = ContentType::new(input);
        assert_eq!(ct.as_str(), expected);
    }

    #[rstest]
    #[case("utf-8", "text/plain; charset=utf-8")]
    #[case("iso-8859-1", "text/plain; charset=iso-8859-1")]
    fn test_charset_directive(#[case] charset: &str, #[case] expected: &str) {
        let ct = ContentType::text().charset(charset);
        assert_eq!(ct.as_str(), expected);
    }

    #[rstest]
    #[case("abc123", "multipart/form-data; boundary=abc123")]
    #[case("my_boundary", "multipart/form-data; boundary=my_boundary")]
    fn test_boundary_directive(#[case] boundary: &str, #[case] expected: &str) {
        let ct = ContentType::multipart(boundary);
        assert_eq!(ct.as_str(), expected);
    }

    #[rstest]
    fn test_arbitrary_directive() {
        let ct = ContentType::json().directive("version", "1.0");
        assert_eq!(ct.as_str(), "application/json; version=1.0");
    }

    #[rstest]
    fn test_convenience_types() {
        assert_eq!(ContentType::text().as_str(), "text/plain");
        assert_eq!(ContentType::html().as_str(), "text/html");
        assert_eq!(ContentType::json().as_str(), "application/json");
        assert_eq!(
            ContentType::octet_stream().as_str(),
            "application/octet-stream"
        );
    }

    #[rstest]
    fn test_default() {
        let default_ct = ContentType::default();
        assert_eq!(default_ct.as_str(), "text/plain");
    }

    #[rstest]
    #[case("text/plain", "charset", "utf-8", "text/plain; charset=utf-8")]
    #[case("application/json", "version", "1.2", "application/json; version=1.2")]
    fn test_push_directive(
        #[case] mime: &str,
        #[case] key: &str,
        #[case] value: &str,
        #[case] expected: &str,
    ) {
        let ct = ContentType::new(mime).directive(key, value);
        assert_eq!(ct.as_str(), expected);
    }

    // Debug-only tests would require #[cfg(debug_assertions)] and panic catching.
    // Example:
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Invalid MIME type")]
    fn test_invalid_mime_type_panics() {
        ContentType::new("invalid-mime"); // Missing '/'
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "charset must not be empty")]
    fn test_empty_charset_panics() {
        ContentType::text().charset("");
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "boundary must not be empty")]
    fn test_empty_boundary_panics() {
        ContentType::multipart("").as_str();
    }
}
