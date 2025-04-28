use super::{decoder::StreamDecoder, headers::HeadersMap};
use crate::{utils::TryClone, IncomingBodyStream};
use bytes::Bytes;
use hyper::Uri;

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
        match value.as_str() {
            "follow" => RequestRedirect::Follow,
            "error" => RequestRedirect::Error,
            "manual" => RequestRedirect::Manual,
            _ => RequestRedirect::Follow,
        }
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

#[derive(Debug)]
pub struct FetchRequest {
    pub method: String,
    pub uri: Uri,
    pub headers: HeadersMap,
    pub keep_alive: bool,
    pub redirect: RequestRedirect,
    pub redirect_count: u32,
    pub body: RequestBody,
}

// Implement TryClone for FetchRequest
impl TryClone for FetchRequest {
    fn try_clone(&self) -> Option<Self> {
        Some(FetchRequest {
            method: self.method.clone(),
            uri: self.uri.clone(),
            headers: self.headers.clone(),
            keep_alive: self.keep_alive,
            redirect: self.redirect,
            redirect_count: self.redirect_count,
            body: self.body.try_clone()?, // return None if body is a stream
        })
    }
}

#[derive(Debug)]
pub struct FetchRequestBuilder {
    method: Option<String>,
    uri: Option<Uri>,
    headers: Option<HeadersMap>,
    keep_alive: Option<bool>,
    redirect: Option<RequestRedirect>,
    redirect_count: Option<u32>,
    body: Option<RequestBody>,
}

impl FetchRequestBuilder {
    pub fn new() -> Self {
        FetchRequestBuilder {
            method: None,
            uri: None,
            headers: Some(HeadersMap::default()),
            keep_alive: Some(true),
            redirect: Some(RequestRedirect::Follow),
            redirect_count: Some(0),
            body: None,
        }
    }

    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    pub fn uri(mut self, uri: impl Into<Uri>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    pub fn headers(mut self, headers: HeadersMap) -> Self {
        self.headers = Some(headers);
        self
    }

    pub fn keep_alive(mut self, keep_alive: bool) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }

    pub fn redirect(mut self, redirect: RequestRedirect) -> Self {
        self.redirect = Some(redirect);
        self
    }

    pub fn body(mut self, body: RequestBody) -> Self {
        self.body = Some(body);
        self
    }

    pub fn build(self) -> Result<FetchRequest, &'static str> {
        Ok(FetchRequest {
            method: self.method.ok_or("Method is required")?,
            uri: self.uri.ok_or("URI is required")?,
            headers: self.headers.unwrap_or_default(),
            keep_alive: self.keep_alive.unwrap_or(true),
            redirect: self.redirect.unwrap_or(RequestRedirect::Follow),
            redirect_count: self.redirect_count.unwrap_or(20),
            body: self.body.unwrap_or(RequestBody::None),
        })
    }
}

pub struct HttpRequest {
    pub method: String,
    pub uri: Uri,
    pub headers: hyper::header::HeaderMap,
    pub keep_alive: bool,
    pub redirect: RequestRedirect,
    pub redirect_count: u32,
    pub stream: Option<IncomingBodyStream>,
}

impl HttpRequest {
    pub fn new(request: hyper::Request<hyper::body::Incoming>) -> Self {
        let parts = request.into_parts();
        let headers = parts.0.headers;
        let uri = parts.0.uri;
        let method = parts.0.method.to_string();
        let keep_alive = false; // TODO: Check if keep-alive is supported
        let redirect = RequestRedirect::Follow; // TODO: Handle redirects
        let redirect_count = 0; // TODO: Handle redirect count

        HttpRequest {
            method,
            uri,
            headers,
            keep_alive,
            redirect,
            redirect_count,
            stream: Some(IncomingBodyStream::new(parts.1)),
        }
    }

    pub fn method(&self) -> String {
        self.method.clone()
    }

    pub fn keep_alive(&self) -> bool {
        self.keep_alive
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    pub fn headers(&self) -> &hyper::header::HeaderMap {
        &self.headers
    }

    pub fn body(&mut self) -> RequestBody {
        let stream = self.stream.take();
        if self.stream.is_none() {
            return RequestBody::None;
        }

        RequestBody::Stream(Some(StreamDecoder::detect_from_headers(
            stream.unwrap(),
            &self.headers,
        )))
    }
}

#[cfg(test)]
mod tests {
    use crate::BoundedBufferChannel;

    use super::*;
    use futures::StreamExt;

    #[test]
    fn test_fetch_builder() {
        let request = FetchRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("https://example.com"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::None)
            .build()
            .unwrap();

        assert_eq!(request.method, "GET");
        assert_eq!(request.uri.to_string(), "https://example.com/");
        assert_eq!(
            request.headers.get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(request.body.is_none(), true);
    }

    #[test]
    fn test_fetch_builder_with_body() {
        let request = FetchRequestBuilder::new()
            .method("POST")
            .uri(Uri::from_static("https://example.com"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::Bytes(Bytes::from(vec![1, 2, 3, 4])))
            .build()
            .unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(request.uri.to_string(), "https://example.com/");
        assert_eq!(
            request.headers.get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(request.body.is_bytes(), true);
        assert_eq!(request.body.is_stream(), false);

        let buffer = match request.body {
            RequestBody::Bytes(buffer) => buffer,
            _ => panic!("Expected a buffer"),
        };
        assert_eq!(buffer, Bytes::from(vec![1, 2, 3, 4]));
    }

    #[tokio::test]
    async fn test_fetch_builder_with_stream() {
        let mut internal_stream = BoundedBufferChannel::new(10);
        internal_stream.try_write(vec![1, 2, 3, 4]).unwrap();

        let request = FetchRequestBuilder::new()
            .method("POST")
            .uri(Uri::from_static("https://example.com"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::Stream(
                internal_stream
                    .aquire_reader()
                    .map(StreamDecoder::internal_stream),
            ))
            .build()
            .unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(request.uri.to_string(), "https://example.com/");
        assert_eq!(
            request.headers.get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(request.body.is_bytes(), false);
        assert_eq!(request.body.is_stream(), true);

        assert!(request.body.try_clone().is_none());

        let mut stream = match request.body {
            RequestBody::Stream(Some(stream)) => stream,
            _ => panic!("Expected a stream"),
        };

        let buffer = stream.next().await.unwrap().unwrap();
        assert_eq!(buffer, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_request_try_clone() {
        let request = FetchRequestBuilder::new()
            .method("POST")
            .uri(Uri::from_static("https://example.com"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::Bytes(Bytes::from(vec![1, 2, 3, 4])))
            .build()
            .unwrap();

        let cloned_request = request.try_clone().unwrap();
        assert_eq!(cloned_request.method, "POST");
        assert_eq!(cloned_request.uri.to_string(), "https://example.com/");
        assert_eq!(
            cloned_request.headers.get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(cloned_request.body.is_bytes(), true);
        assert_eq!(cloned_request.body.is_stream(), false);

        let buffer = match cloned_request.body {
            RequestBody::Bytes(buffer) => buffer,
            _ => panic!("Expected a buffer"),
        };
        assert_eq!(buffer, Bytes::from(vec![1, 2, 3, 4]));
    }
}
