use super::decoder::StreamDecoder;
use crate::{utils::TryClone, IncomingBodyStream};
use bytes::Bytes;
use hyper::{header::HeaderMap, Uri};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequestRedirect {
    Follow,
    Error,
    Manual,
}

impl From<&str> for RequestRedirect {
    fn from(value: &str) -> Self {
        match value {
            "follow" => RequestRedirect::Follow,
            "error" => RequestRedirect::Error,
            "manual" => RequestRedirect::Manual,
            _ => RequestRedirect::Follow,
        }
    }
}

impl From<String> for RequestRedirect {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<u8> for RequestRedirect {
    fn from(value: u8) -> Self {
        match value {
            0 => RequestRedirect::Follow,
            1 => RequestRedirect::Error,
            2 => RequestRedirect::Manual,
            _ => RequestRedirect::Follow,
        }
    }
}

impl Into<u8> for &RequestRedirect {
    fn into(self) -> u8 {
        match self {
            RequestRedirect::Follow => 0,
            RequestRedirect::Error => 1,
            RequestRedirect::Manual => 2,
        }
    }
}

/// Body content for HTTP requests
#[derive(Debug)]
pub enum RequestBody {
    None,
    Bytes(Bytes),
    Stream(Option<StreamDecoder>),
}

impl RequestBody {
    pub fn is_none(&self) -> bool {
        matches!(self, RequestBody::None)
    }

    pub fn is_bytes(&self) -> bool {
        matches!(self, RequestBody::Bytes(_))
    }

    pub fn is_stream(&self) -> bool {
        matches!(self, RequestBody::Stream(_))
    }

    pub fn is_none_stream(&self) -> bool {
        matches!(self, RequestBody::Stream(None))
    }

    pub fn take(&mut self) -> Self {
        std::mem::replace(self, RequestBody::None)
    }
}

impl TryClone for RequestBody {
    fn try_clone(&self) -> Option<Self> {
        match self {
            RequestBody::None => Some(RequestBody::None),
            RequestBody::Bytes(bytes) => Some(RequestBody::Bytes(bytes.clone())),
            RequestBody::Stream(_) => None, // Cannot clone streams
        }
    }
}

impl Clone for RequestBody {
    fn clone(&self) -> Self {
        match self {
            RequestBody::None => RequestBody::None,
            RequestBody::Bytes(bytes) => RequestBody::Bytes(bytes.clone()),
            RequestBody::Stream(_) => RequestBody::Stream(None),
        }
    }
}

/// A unified HTTP request representation used by both client and server
///
/// This struct represents both outgoing requests (client) via `Fetch` API
/// and incoming requests (server) via `Server` API.
#[derive(Debug)]
pub struct HttpRequest {
    /// HTTP method (GET, POST, etc.)
    method: Cow<'static, str>,
    /// Request URI
    uri: Uri,
    /// HTTP headers
    headers: HeaderMap,
    /// Whether to use keep-alive
    keep_alive: bool,
    /// Redirect behavior
    redirect: RequestRedirect,
    /// Redirect count (prevents infinite redirects)
    redirect_count: u16,
    /// Request body content (outgoing)
    body: RequestBody,
    /// Request body stream (incoming)
    stream: Option<IncomingBodyStream>,
}

impl HttpRequest {
    /// Create a new HttpRequest from hyper components
    pub fn new(request: hyper::Request<hyper::body::Incoming>) -> Self {
        let (parts, incoming) = request.into_parts();

        Self {
            method: Cow::Owned(parts.method.to_string()),
            uri: parts.uri,
            headers: parts.headers,
            keep_alive: false, // Default to false for incoming requests
            redirect: RequestRedirect::Follow,
            redirect_count: 0,
            body: RequestBody::None,
            stream: Some(IncomingBodyStream::new(incoming)),
        }
    }

    /// Get the HTTP method
    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn set_method<S>(&mut self, method: S)
    where
        S: Into<Cow<'static, str>>,
    {
        self.method = method.into();
    }

    /// Get the keep-alive status
    pub fn keep_alive(&self) -> bool {
        self.keep_alive
    }

    /// Get the request URI
    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    pub fn set_uri(&mut self, uri: Uri) {
        self.uri = uri;
    }

    pub fn redirect(&self) -> &RequestRedirect {
        &self.redirect
    }

    pub fn redirect_count(&self) -> u16 {
        self.redirect_count
    }

    /// Get the request headers
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    pub fn set_body(&mut self, body: RequestBody) {
        self.body = body;
    }

    pub fn body_mut(&mut self) -> &mut RequestBody {
        if self.stream.is_some() {
            let stream = self.stream.take().unwrap();
            self.body = RequestBody::Stream(Some(StreamDecoder::detect_encoding(
                stream,
                &self.headers,
            )));
        }

        &mut self.body
    }

    /// Take the body stream, creating an appropriate decoder based on Content-Encoding
    pub fn take_body(&mut self) -> RequestBody {
        // For outgoing requests
        if !self.body.is_none() {
            return self.body.take();
        }

        // For incoming requests
        let stream = self.stream.take();
        if stream.is_none() {
            return RequestBody::None;
        }

        RequestBody::Stream(Some(StreamDecoder::detect_encoding(
            stream.unwrap(),
            &self.headers,
        )))
    }

    /// Check if the request has a body stream
    pub fn has_stream(&self) -> bool {
        self.stream.is_some()
    }
}

impl TryClone for HttpRequest {
    fn try_clone(&self) -> Option<Self> {
        Some(HttpRequest {
            method: self.method.clone(),
            uri: self.uri.clone(),
            headers: self.headers.clone(),
            keep_alive: self.keep_alive,
            redirect: self.redirect,
            redirect_count: self.redirect_count,
            body: self.body.try_clone()?, // return None if body is a stream
            stream: None,                 // Streams can't be cloned
        })
    }
}

#[derive(Debug)]
pub struct HttpRequestBuilder {
    method: Option<Cow<'static, str>>,
    uri: Option<Uri>,
    headers: Option<HeaderMap>,
    keep_alive: Option<bool>,
    redirect: Option<RequestRedirect>,
    redirect_count: Option<u16>,
    body: Option<RequestBody>,
}

impl HttpRequestBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        HttpRequestBuilder {
            method: None,
            uri: None,
            headers: Some(HeaderMap::default()),
            keep_alive: Some(true),
            redirect: Some(RequestRedirect::Follow),
            redirect_count: Some(0),
            body: None,
        }
    }

    /// Set the HTTP method
    pub fn method<S>(mut self, method: S) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        self.method = Some(method.into());
        self
    }

    /// Set the request URI
    pub fn uri(mut self, uri: impl Into<Uri>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    /// Set the HTTP headers
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Set the keep-alive flag
    pub fn keep_alive(mut self, keep_alive: bool) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }

    /// Set the redirect behavior
    pub fn redirect(mut self, redirect: RequestRedirect) -> Self {
        self.redirect = Some(redirect);
        self
    }

    /// Set the redirect count
    pub fn redirect_count(mut self, count: u16) -> Self {
        self.redirect_count = Some(count);
        self
    }

    /// Set the request body
    pub fn body(mut self, body: RequestBody) -> Self {
        self.body = Some(body);
        self
    }

    /// Build the HttpRequest
    pub fn build(self) -> Result<HttpRequest, &'static str> {
        Ok(HttpRequest {
            method: self.method.ok_or("Method is required")?,
            uri: self.uri.ok_or("URI is required")?,
            headers: self.headers.unwrap_or_default(),
            keep_alive: self.keep_alive.unwrap_or(true),
            redirect: self.redirect.unwrap_or(RequestRedirect::Follow),
            redirect_count: self.redirect_count.unwrap_or(20),
            body: self.body.unwrap_or(RequestBody::None),
            stream: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::BoundedBufferChannel;

    use super::*;
    use futures::StreamExt;

    #[test]
    fn test_request_builder() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        let mut request = HttpRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("https://example.com"))
            .headers(headers)
            .body(RequestBody::None)
            .build()
            .unwrap();

        assert_eq!(request.method(), "GET");
        assert_eq!(request.uri().to_string(), "https://example.com/");
        assert_eq!(
            request.headers().get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(matches!(request.take_body(), RequestBody::None), true);
    }

    #[test]
    fn test_request_builder_with_body() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        let mut request = HttpRequestBuilder::new()
            .method("POST")
            .uri(Uri::from_static("https://example.com"))
            .headers(headers)
            .body(RequestBody::Bytes(Bytes::from(vec![1, 2, 3, 4])))
            .build()
            .unwrap();

        assert_eq!(request.method(), "POST");
        assert_eq!(request.uri().to_string(), "https://example.com/");
        assert_eq!(
            request.headers().get("Content-Type").unwrap(),
            "application/json"
        );

        let body = request.take_body();
        assert_eq!(body.is_bytes(), true);
        assert_eq!(body.is_stream(), false);

        if let RequestBody::Bytes(buffer) = body {
            assert_eq!(buffer, Bytes::from(vec![1, 2, 3, 4]));
        } else {
            panic!("Expected a buffer");
        }
    }

    #[tokio::test]
    async fn test_request_builder_with_stream() {
        let mut internal_stream = BoundedBufferChannel::new(10);
        internal_stream.try_write(vec![1, 2, 3, 4]).unwrap();

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("Transfer-Encoding", "chunked".parse().unwrap());
        let mut request = HttpRequestBuilder::new()
            .method("POST")
            .uri(Uri::from_static("https://example.com"))
            .headers(headers)
            .body(RequestBody::Stream(
                internal_stream
                    .aquire_reader()
                    .map(StreamDecoder::internal_stream),
            ))
            .build()
            .unwrap();

        assert_eq!(request.method(), "POST");
        assert_eq!(request.uri().to_string(), "https://example.com/");
        assert_eq!(
            request.headers().get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(
            request.headers().get("Transfer-Encoding").unwrap(),
            "chunked"
        );

        let body = request.take_body();
        assert_eq!(body.is_bytes(), false);
        assert_eq!(body.is_stream(), true);

        if let RequestBody::Stream(Some(mut stream)) = body {
            let buffer = stream.next().await.unwrap().unwrap();
            assert_eq!(buffer, vec![1, 2, 3, 4]);
        } else {
            panic!("Expected a stream");
        }
    }

    #[test]
    fn test_request_try_clone() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        let request = HttpRequestBuilder::new()
            .method("POST")
            .uri(Uri::from_static("https://example.com"))
            .headers(headers)
            .body(RequestBody::Bytes(Bytes::from(vec![1, 2, 3, 4])))
            .build()
            .unwrap();

        let cloned_request = request.try_clone().unwrap();
        assert_eq!(cloned_request.method(), "POST");
        assert_eq!(cloned_request.uri().to_string(), "https://example.com/");
        assert_eq!(
            cloned_request.headers().get("Content-Type").unwrap(),
            "application/json"
        );

        if let RequestBody::Bytes(buffer) = cloned_request.body {
            assert_eq!(buffer, Bytes::from(vec![1, 2, 3, 4]));
        } else {
            panic!("Expected a buffer");
        }
    }

    #[test]
    fn test_method_static_string() {
        // Test that static strings don't allocate
        let req = HttpRequestBuilder::new()
            .method("GET") // Should use Cow::Borrowed
            .uri("https://example.com".parse::<Uri>().unwrap())
            .build()
            .unwrap();

        assert_eq!(req.method(), "GET");

        // Test with owned string
        let method = String::from("POST");
        let req = HttpRequestBuilder::new()
            .method(method) // Should use Cow::Owned
            .uri("https://example.com".parse::<Uri>().unwrap())
            .build()
            .unwrap();

        assert_eq!(req.method(), "POST");
    }
}
