use super::{body::HttpBody, decoder::StreamDecoder, encoder::StreamEncoder};
use crate::FetchError;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use hyper::{header::HeaderMap, StatusCode, Uri};
use std::borrow::Cow;

#[derive(Debug)]
pub enum ResponseBody {
    None,
    Bytes(Bytes),
    // Stream(InternalBodyStream),
    DecodedStream(StreamDecoder),
    EncodedStream(StreamEncoder),
}

impl ResponseBody {
    /// Check if this body is empty
    pub fn is_none(&self) -> bool {
        matches!(self, ResponseBody::None)
    }

    /// Check if this body contains raw bytes
    pub fn is_bytes(&self) -> bool {
        matches!(self, ResponseBody::Bytes(_))
    }

    /// Check if this body contains a decoded stream
    pub fn is_decoded_stream(&self) -> bool {
        matches!(self, ResponseBody::DecodedStream(_))
    }

    /// Check if this body contains an encoded stream
    pub fn is_encoded_stream(&self) -> bool {
        matches!(self, ResponseBody::EncodedStream(_))
    }
}

impl TryInto<hyper::Response<HttpBody>> for HttpResponse {
    type Error = FetchError;

    fn try_into(self) -> Result<hyper::Response<HttpBody>, Self::Error> {
        let body = match self.body {
            ResponseBody::EncodedStream(stream) => stream.boxed(),
            ResponseBody::DecodedStream(stream) => stream.boxed(),
            ResponseBody::None => Empty::new()
                .map_err(|_| FetchError::new("Http Empty Body Error"))
                .boxed(),
            ResponseBody::Bytes(bytes) => Full::new(bytes)
                .map_err(|_| FetchError::new("Http Full Body Error"))
                .boxed(),
        };

        let builder = hyper::Response::builder().status(self.status);
        let mut response = builder
            .body(body)
            .map_err(|e| FetchError::new(&format!("Failed to build response: {}", e)))?;

        *response.headers_mut() = self.headers;
        Ok(response)
    }
}

#[derive(Debug)]
pub struct HttpResponse {
    /// The URLs involved in this response (including redirects)
    urls: Vec<Uri>,
    /// HTTP status code
    status: StatusCode,
    /// HTTP headers
    headers: HeaderMap,
    /// Whether the request was aborted
    aborted: bool,
    /// Response body
    body: ResponseBody,
}

impl HttpResponse {
    /// Create a new HTTP response
    pub fn new(status: StatusCode, headers: HeaderMap, body: ResponseBody) -> Self {
        Self {
            urls: Vec::with_capacity(1), // Optimize for common case of single URL
            status,
            headers,
            aborted: false,
            body,
        }
    }

    /// Add a URL to the response history
    pub fn add_url(&mut self, url: Uri) -> &mut Self {
        self.urls.push(url);
        self
    }

    /// Set the URLs list
    pub fn set_urls(&mut self, urls: Vec<Uri>) -> &mut Self {
        self.urls = urls;
        self
    }

    /// Get all URLs involved in this response
    pub fn urls(&self) -> &[Uri] {
        &self.urls
    }

    /// Get the final URL (last one in the chain)
    pub fn url(&self) -> Option<&Uri> {
        self.urls.last()
    }

    /// Get the HTTP status code
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Get the numeric status code
    pub fn status_code(&self) -> u16 {
        self.status.as_u16()
    }

    /// Get the status text (avoids allocating for common statuses)
    pub fn status_text(&self) -> Cow<'static, str> {
        self.status
            .canonical_reason()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(format!("Status {}", self.status.as_u16())))
    }

    /// Get the HTTP headers
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get mutable reference to headers
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Check if the request was aborted
    pub fn is_aborted(&self) -> bool {
        self.aborted
    }

    /// Mark the request as aborted
    pub fn set_aborted(&mut self, aborted: bool) -> &mut Self {
        self.aborted = aborted;
        self
    }

    /// Get the response body
    pub fn body(&self) -> &ResponseBody {
        &self.body
    }

    /// Take the response body, replacing it with None
    pub fn take_body(&mut self) -> ResponseBody {
        std::mem::replace(&mut self.body, ResponseBody::None)
    }

    /// Set the response body
    pub fn set_body(&mut self, body: ResponseBody) -> &mut Self {
        self.body = body;
        self
    }

    /// Create a response with an empty body
    pub fn empty(status: StatusCode) -> Self {
        Self::new(status, HeaderMap::new(), ResponseBody::None)
    }

    /// Create a 200 OK response with the given body
    pub fn ok(body: ResponseBody) -> Self {
        Self::new(StatusCode::OK, HeaderMap::new(), body)
    }

    /// Create a 404 Not Found response
    pub fn not_found() -> Self {
        Self::empty(StatusCode::NOT_FOUND)
    }

    /// Create a response from bytes with a 200 OK status
    pub fn from_bytes(bytes: Bytes) -> Self {
        Self::ok(ResponseBody::Bytes(bytes))
    }

    /// Create a response from a string with a 200 OK status
    pub fn from_string(s: String) -> Self {
        Self::from_bytes(Bytes::from(s))
    }

    pub fn builder() -> HttpResponseBuilder {
        HttpResponseBuilder::new()
    }
}

/// Builder for HttpResponse
#[derive(Debug, Default)]
pub struct HttpResponseBuilder {
    urls: Option<Vec<Uri>>,
    status: Option<StatusCode>,
    headers: Option<HeaderMap>,
    aborted: Option<bool>,
    body: Option<ResponseBody>,
}

impl HttpResponseBuilder {
    /// Create a new response builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the response status
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the response status from a u16 code
    pub fn status_code(mut self, code: u16) -> Result<Self, &'static str> {
        self.status =
            Some(StatusCode::from_u16(code).map_err(|_| "Invalid status code")?);
        Ok(self)
    }

    /// Set the response headers
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Set a single header value
    pub fn header(mut self, name: &str, value: &str) -> Result<Self, &'static str> {
        use hyper::header::{HeaderName, HeaderValue};
        let headers = self.headers.get_or_insert_with(HeaderMap::new);
        headers.insert(
            HeaderName::from_bytes(name.as_bytes()).map_err(|_| "Invalid header name")?,
            HeaderValue::from_str(value).map_err(|_| "Invalid header value")?,
        );
        Ok(self)
    }

    /// Set the response body
    pub fn body(mut self, body: ResponseBody) -> Self {
        self.body = Some(body);
        self
    }

    /// Set the response body from bytes
    pub fn bytes_body(mut self, bytes: Bytes) -> Self {
        self.body = Some(ResponseBody::Bytes(bytes));
        self
    }

    /// Set the response body from a string
    pub fn string_body(self, s: String) -> Self {
        self.bytes_body(Bytes::from(s))
    }

    /// Set the response URLs
    pub fn urls(mut self, urls: Vec<Uri>) -> Self {
        self.urls = Some(urls);
        self
    }

    /// Add a URL to the response history
    pub fn url(mut self, url: Uri) -> Self {
        let urls = self.urls.get_or_insert_with(|| Vec::with_capacity(1));
        urls.push(url);
        self
    }

    /// Set the aborted flag
    pub fn aborted(mut self, aborted: bool) -> Self {
        self.aborted = Some(aborted);
        self
    }

    /// Build the HTTP response
    pub fn build(self) -> Result<HttpResponse, &'static str> {
        let mut response = HttpResponse {
            urls: self.urls.unwrap_or_else(Vec::new),
            status: self.status.ok_or("Status code is required")?,
            headers: self.headers.unwrap_or_default(),
            aborted: self.aborted.unwrap_or(false),
            body: self.body.unwrap_or(ResponseBody::None),
        };

        // If there are no URLs but we have a response, default to an empty URL
        if response.urls.is_empty() {
            response.urls.push(Uri::from_static(""));
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_builder() {
        let response = HttpResponseBuilder::new()
            .status(StatusCode::OK)
            .url(Uri::from_static("https://example.com"))
            .header("Content-Type", "text/plain")
            .unwrap()
            .bytes_body(Bytes::from("Hello, world!"))
            .build()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_text(), "OK");
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            "text/plain"
        );
        assert!(!response.is_aborted());

        if let ResponseBody::Bytes(bytes) = response.body() {
            assert_eq!(bytes, &Bytes::from("Hello, world!"));
        } else {
            panic!("Expected bytes body");
        }
    }

    #[test]
    fn test_response_methods() {
        let mut response = HttpResponse::new(
            StatusCode::NOT_FOUND,
            HeaderMap::new(),
            ResponseBody::None,
        );

        response.add_url(Uri::from_static("https://example.com/page1"));
        response.add_url(Uri::from_static("https://example.com/page2"));

        assert_eq!(response.urls().len(), 2);
        assert_eq!(
            response.url().unwrap().to_string(),
            "https://example.com/page2"
        );
        assert_eq!(response.status_text(), "Not Found");

        // Test custom status without canonical reason
        let custom_response = HttpResponseBuilder::new()
            .status_code(599)
            .unwrap()
            .url(Uri::from_static("https://example.com"))
            .build()
            .unwrap();

        assert_eq!(custom_response.status_text(), "Status 599");
    }

    #[test]
    fn test_response_conversion() {
        let response = HttpResponseBuilder::new()
            .status(StatusCode::OK)
            .url(Uri::from_static("https://example.com"))
            .bytes_body(Bytes::from("Hello"))
            .build()
            .unwrap();

        let hyper_response: Result<hyper::Response<HttpBody>, _> = response.try_into();
        assert!(hyper_response.is_ok());

        let hyper_response = hyper_response.unwrap();
        assert_eq!(hyper_response.status(), StatusCode::OK);
    }

    #[test]
    fn test_convenience_constructors() {
        let ok_response = HttpResponse::ok(ResponseBody::None);
        assert_eq!(ok_response.status(), StatusCode::OK);

        let not_found = HttpResponse::not_found();
        assert_eq!(not_found.status(), StatusCode::NOT_FOUND);

        let bytes_response = HttpResponse::from_bytes(Bytes::from("test"));
        assert_eq!(bytes_response.status(), StatusCode::OK);
        assert!(matches!(bytes_response.body(), ResponseBody::Bytes(_)));

        let string_response = HttpResponse::from_string("test".to_string());
        assert_eq!(string_response.status(), StatusCode::OK);
        assert!(matches!(string_response.body(), ResponseBody::Bytes(_)));
    }
}
