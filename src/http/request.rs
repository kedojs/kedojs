use bytes::Bytes;
use hyper::Uri;
use rust_jsc::{JSArray, JSContext, JSError, JSObject, JSResult, JSTypedArray, JSValue};

use crate::{
    streams::streams::{InternalStreamResource, InternalStreamResourceReader},
    traits::TryClone,
    utils::downcast_ref,
};

use super::HeadersMap;

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
    Stream(Option<InternalStreamResourceReader<Vec<u8>>>),
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
            body: self.body.try_clone()?, // Fails if body cannot be cloned
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
            redirect_count: Some(20),
            body: Some(RequestBody::None),
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

impl TryInto<RequestBody> for JSObject {
    type Error = JSError;

    fn try_into(self) -> Result<RequestBody, Self::Error> {
        if self.has_property("source") {
            let source = self.get_property("source")?.as_object()?;
            let buffer = JSTypedArray::from(source);
            Ok(RequestBody::Bytes(Bytes::from(buffer.as_vec()?)))
        } else if self.has_property("stream") {
            let stream = downcast_ref::<InternalStreamResource<Vec<u8>>>(
                &self.get_property("stream")?.as_object()?,
            );
            Ok(stream
                .map(|mut s| RequestBody::Stream(s.as_mut().new_reader()))
                .unwrap_or(RequestBody::None))
        } else {
            Ok(RequestBody::None)
        }
    }
}

impl FetchRequest {
    pub fn from_value(value: &JSValue, ctx: &JSContext) -> JSResult<Self> {
        if !value.is_object() {
            return Err(JSError::new_typ(&ctx, "Missing highWaterMark argument")?);
        }

        let request = value.as_object()?;
        let method = request.get_property("method")?.as_string()?.to_string();
        let url = request.get_property("url")?.as_string()?.to_string();
        let redirect = request
            .get_property("redirect")
            .and_then(|v| Ok(RequestRedirect::from(v.as_number()? as u8)))
            .unwrap_or(RequestRedirect::Follow);
        let keep_alive = request
            .get_property("keep_alive")
            .and_then(|v| Ok(v.as_boolean()))
            .unwrap_or(false);
        let uri = url
            .parse::<hyper::Uri>()
            .map_err(|_| JSError::new_typ(&ctx, "Invalid URL").unwrap())?;
        let header_list = request.get_property("header_list")?.as_object()?;
        if !header_list.is_array() {
            return Err(JSError::new_typ(&ctx, "header_list must be an array")?);
        }

        let headers = HeadersMap::from(JSArray::new(header_list));
        let body = request.try_into()?;
        let request = FetchRequestBuilder::new()
            .method(method)
            .uri(uri)
            .headers(headers)
            .keep_alive(keep_alive)
            .redirect(redirect)
            .body(body)
            .build()
            .map_err(|e| JSError::new_typ(&ctx, e).unwrap())?;

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::test_utils::new_runtime;

    #[test]
    fn test_request_initialization() {
        let rt = new_runtime();
        let result = rt.evaluate_script(
            r#"
            const request_params = {
                method: 'GET',
                url: 'https://example.com',
                header_list: [['Content-Type', 'application/json']]
            };
            request_params
        "#,
            None,
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        let request = FetchRequest::from_value(&result, rt.context()).unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.uri.to_string(), "https://example.com/");
        assert_eq!(
            request.headers.get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(request.body.is_none(), true);
    }

    #[test]
    fn test_request_initialization_with_body() {
        let rt = new_runtime();
        let result = rt.evaluate_script(
            r#"
            const request_params = {
                method: 'POST',
                url: 'https://example.com',
                header_list: [['Content-Type', 'application/json']],
                source: new Uint8Array([1, 2, 3, 4])
            };
            request_params
        "#,
            None,
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        let request = FetchRequest::from_value(&result, rt.context()).unwrap();
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
    async fn test_request_initialization_with_stream() {
        let mut rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { ReadableStream, readableStreamResource } from "@kedo/stream";
            const request_params = {
                method: 'POST',
                url: 'https://example.com',
                header_list: [['Content-Type', 'application/json']],
                stream: readableStreamResource(new ReadableStream({
                    start(controller) {
                        controller.enqueue(new Uint8Array([1, 2, 3, 4]));
                        controller.close();
                    }
                }))
            };
            globalThis.request_params = request_params;
            setTimeout(() => {
                console.log("Done");
            }, 1000);
        "#,
            "index.js",
            None,
        );

        assert!(result.is_ok());
        let result = rt
            .evaluate_script("globalThis.request_params", None)
            .unwrap();
        let request = FetchRequest::from_value(&result, rt.context()).unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.uri.to_string(), "https://example.com/");
        assert_eq!(
            request.headers.get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(request.body.is_bytes(), false);
        assert_eq!(request.body.is_stream(), true);
        // consume the stream
        let mut stream = match request.body {
            RequestBody::Stream(Some(stream)) => stream,
            _ => panic!("Expected a stream"),
        };

        rt.idle().await;
        let mut buffer = vec![];
        while let Some(chunk) = stream.read_async().await.unwrap() {
            buffer.extend_from_slice(&chunk);
        }
        assert_eq!(buffer, vec![1, 2, 3, 4]);
    }

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

    #[test]
    fn test_fetch_builder_with_stream() {
        let mut internal_stream = InternalStreamResource::new(10);
        internal_stream.write(vec![1, 2, 3, 4]).unwrap();

        let request = FetchRequestBuilder::new()
            .method("POST")
            .uri(Uri::from_static("https://example.com"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::Stream(internal_stream.new_reader()))
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

        let buffer = stream.read().unwrap();
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
