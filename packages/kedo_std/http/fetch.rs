use super::errors::FetchError;
use crate::http::body::IncomingBodyStream;
use crate::http::decoder::StreamDecoder;
use crate::http::headers::HeadersMap;
use crate::http::request::{FetchRequest, RequestBody, RequestRedirect};
use crate::http::response::{FetchResponse, ResponseBody};
use crate::http::utils::{basic_auth, extract_authority, remove_credentials};
use futures::ready;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Either, Empty, Full};
use hyper::header::{
    AUTHORIZATION, COOKIE, LOCATION, PROXY_AUTHORIZATION, WWW_AUTHENTICATE,
};
use hyper::{Method, Request, StatusCode, Uri};
use hyper_tls::HttpsConnector;
use hyper_util::client::legacy::ResponseFuture;
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use hyper_util::rt::TokioExecutor;
use std::convert::Infallible;
use std::error::Error;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

type FetchBodyStream = StreamDecoder;
type FetchBody = Either<FetchBodyStream, BoxBody<bytes::Bytes, Infallible>>;

/// | ------------------------------- |
/// |           FetchClient           |
/// | ------------------------------- |
/// | - hyper: Client                 |
/// | - max_redirects: u32            |
/// | ------------------------------- |
///
/// This is the main entry point for making HTTP requests.
#[derive(Debug, Clone)]
pub struct FetchClient {
    hyper: Client<HttpsConnector<HttpConnector>, FetchBody>,
    max_redirects: u32,
}

impl TryFrom<&mut FetchRequest> for Request<FetchBody> {
    type Error = Box<dyn Error>;

    fn try_from(fetch_request: &mut FetchRequest) -> Result<Self, Self::Error> {
        let mut uri = fetch_request.uri.clone();
        let auth = extract_authority(&uri);
        if let Some((username, password)) = auth {
            let basic_auth = basic_auth(username.as_str(), password.as_deref());
            fetch_request
                .headers
                .append(AUTHORIZATION.as_str(), basic_auth.as_str());
            uri = remove_credentials(&uri);
        }

        let mut builder = Request::builder()
            .method(fetch_request.method.as_str())
            .uri(uri);

        // Set headers
        for (name, value) in fetch_request.headers.into_inner() {
            builder = builder.header(name.as_str(), value.as_str());
        }

        // Set body
        let body = match &mut fetch_request.body {
            RequestBody::None => Either::Right(Empty::new().boxed()),
            RequestBody::Bytes(bytes) => Either::Right(Full::new(bytes.clone()).boxed()),
            RequestBody::Stream(stream) => match stream.take() {
                Some(stream) => Either::Left(stream),
                None => Either::Right(Empty::new().boxed()),
            },
        };

        let request = builder.body(body)?;
        Ok(request)
    }
}

pub struct PendingRequest {
    fetch_request: FetchRequest,
    in_flight: ResponseFuture,
    urls: Vec<Uri>,
    redirect_count: u32,
    client: Arc<FetchClient>,
}

impl PendingRequest {
    fn must_follow_redirect(&self) -> bool {
        self.fetch_request.redirect == RequestRedirect::Follow
    }

    fn error_on_redirect(&self) -> bool {
        self.fetch_request.redirect == RequestRedirect::Error
    }

    fn too_many_redirects(&self) -> bool {
        self.redirect_count >= self.client.max_redirects
    }
}

impl Future for PendingRequest {
    type Output = Result<FetchResponse, FetchError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let response =
            ready!(Pin::new(&mut self.in_flight).poll(cx)).map_err(FetchError::from)?;
        let is_redirect = self.client.is_redirect(response.status());
        // Follow redirect if necessary
        if is_redirect && self.too_many_redirects() {
            return Poll::Ready(Err(FetchError {
                message: "Too many redirects".into(),
                inner: None,
            }));
        }

        if is_redirect && self.must_follow_redirect() {
            let is_see_other = response.status() == StatusCode::SEE_OTHER;
            if self.fetch_request.body.is_none_stream() && !is_see_other {
                return Poll::Ready(Err(FetchError {
                    message: "Redirect cannot be followed with stream body".into(),
                    inner: None,
                }));
            }

            if let Some(location_header) = response.headers().get(LOCATION) {
                let location = location_header.to_str().map_err(FetchError::from)?;
                let new_uri = self
                    .client
                    .resolve_redirect(&self.fetch_request.uri, location)
                    .map_err(|error| FetchError {
                        message: error.to_string(),
                        inner: None,
                    })?;

                let fetch_request_mut = &mut self.fetch_request;
                FetchClient::remove_sensitive_headers(fetch_request_mut, &new_uri);

                self.fetch_request.uri = new_uri.clone();
                self.urls.push(new_uri);
                self.redirect_count += 1;

                if is_see_other {
                    self.fetch_request.method = Method::GET.to_string();
                    self.fetch_request.body = RequestBody::None;
                }

                let hyper_request =
                    Request::try_from(&mut self.fetch_request).map_err(|e| {
                        FetchError {
                            message: e.to_string(),
                            inner: None,
                        }
                    })?;
                self.in_flight = self.client.hyper.request(hyper_request);
                return self.poll(cx);
            } else {
                return Poll::Ready(Err(FetchError {
                    message: "Redirect status but no Location header".into(),
                    inner: None,
                }));
            }
        } else if is_redirect && self.error_on_redirect() {
            return Poll::Ready(Err(FetchError {
                message: "Redirect encountered but redirection not allowed".into(),
                inner: None,
            }));
        }

        let status = response.status().as_u16();
        let status_message = response
            .status()
            .canonical_reason()
            .unwrap_or("")
            .to_string();

        let mut headers = HeadersMap::default();
        for (name, value) in response.headers() {
            headers.append(
                name.as_str(),
                value.to_str().map_err(|error| FetchError {
                    message: "Invalid header value".into(),
                    inner: Some(error.into()),
                })?,
            );
        }

        let body_stream = IncomingBodyStream::new(response.into_body());
        let decoder_body = StreamDecoder::detect(body_stream, &headers);
        let decoder_body = ResponseBody::DecodedStream(decoder_body);
        let fetch_response = FetchResponse {
            urls: self.urls.clone(),
            status,
            status_message,
            headers,
            aborted: false,
            body: decoder_body,
        };

        Poll::Ready(Ok(fetch_response))
    }
}

impl FetchClient {
    pub fn new() -> Self {
        // TODO: enable this by feature flag
        let tls_connector = tokio_native_tls::native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .build()
            .expect("Failed to create TLS connector")
            .into();

        let mut http_connector = HttpConnector::new();
        http_connector.enforce_http(false);
        let https = HttpsConnector::from((http_connector, tls_connector));
        let hyper = Client::builder(TokioExecutor::new()).build(https);
        FetchClient {
            hyper,
            max_redirects: 20,
        }
    }

    pub fn execute(
        &self,
        mut request: FetchRequest,
    ) -> Result<PendingRequest, FetchError> {
        let urls = vec![request.uri.clone()];
        let hyper_request = Request::try_from(&mut request).map_err(|e| FetchError {
            message: e.to_string(),
            inner: None,
        })?;
        let in_flight = self.hyper.request(hyper_request);
        Ok(PendingRequest {
            fetch_request: request,
            in_flight,
            urls,
            redirect_count: 0,
            client: Arc::new(self.clone()),
        })
    }

    fn is_redirect(&self, status: StatusCode) -> bool {
        matches!(
            status,
            StatusCode::MOVED_PERMANENTLY
                | StatusCode::FOUND
                | StatusCode::SEE_OTHER
                | StatusCode::TEMPORARY_REDIRECT
                | StatusCode::PERMANENT_REDIRECT
        )
    }

    fn resolve_redirect(
        &self,
        base_uri: &Uri,
        location: &str,
    ) -> Result<Uri, Box<dyn Error>> {
        let location_uri = location.parse::<Uri>()?;
        let new_uri =
            if location_uri.scheme().is_some() && location_uri.authority().is_some() {
                location_uri
            } else {
                let mut parts = base_uri.clone().into_parts();
                if let Some(path_and_query) = location_uri.path_and_query() {
                    parts.path_and_query = Some(path_and_query.clone());
                }
                Uri::from_parts(parts)?
            };
        Ok(new_uri)
    }

    pub(crate) fn remove_sensitive_headers(request: &mut FetchRequest, next: &Uri) {
        let previous = &request.uri;
        // check host and port
        let cross_host = previous.host() != next.host() || previous.port() != next.port();
        if cross_host {
            request.headers.remove(AUTHORIZATION.as_str());
            request.headers.remove(COOKIE.as_str());
            request.headers.remove(PROXY_AUTHORIZATION.as_str());
            request.headers.remove(WWW_AUTHENTICATE.as_str());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::request::FetchRequestBuilder;
    use crate::http::request::{RequestBody, RequestRedirect};
    use crate::http::tests::test_utils::start_test_server;
    use crate::BoundedBufferChannel;
    use bytes::Bytes;
    use futures::stream::StreamExt;
    use futures::TryStreamExt;
    use hyper::{body, Response};

    fn build_redirect_response(
        location: &str,
        status: StatusCode,
    ) -> Response<Full<Bytes>> {
        Response::builder()
            .status(status)
            .header(LOCATION, location)
            .body(Full::from(Bytes::from_static(b"")))
            .unwrap()
    }

    async fn handler_redirect(
        req: Request<body::Incoming>,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        let redirect_status = req
            .headers()
            .get("X-Redirect-Status")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u16>().ok())
            .map(StatusCode::from_u16)
            .unwrap_or(Ok(StatusCode::SEE_OTHER))
            .unwrap_or(StatusCode::SEE_OTHER);

        match req.uri().path() {
            "/" => Ok(build_redirect_response("/1", redirect_status)),
            "/1" => Ok(build_redirect_response("/2", redirect_status)),
            "/many" => Ok(build_redirect_response("/many", redirect_status)),
            "/2" => Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Full::from(Bytes::from_static(b"Hello, world!")))
                .unwrap()),
            _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::from(Bytes::from_static(b"")))
                .unwrap()),
        }
    }

    #[tokio::test]
    async fn test_fetch_client() {
        let client = FetchClient::new();
        let request = FetchRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("https://httpbin.org/get"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::None)
            .build()
            .unwrap();

        let response = client.execute(request).unwrap().await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.status_message, "OK");
        assert_eq!(response.urls[0], "https://httpbin.org/get");
        assert_eq!(response.aborted, false);

        let body = match response.body {
            ResponseBody::DecodedStream(stream) => stream,
            _ => panic!("Expected body"),
        };

        let mut body = body.into_stream();
        let mut buffer = Vec::new();
        while let Some(chunk) = body.next().await {
            buffer.extend_from_slice(&chunk.unwrap());
        }
        let body = String::from_utf8(buffer).unwrap();

        assert!(body.contains("httpbin.org"));
        assert!(body.contains("https://httpbin.org/get"));
    }

    #[tokio::test]
    async fn test_fetch_redirect() {
        let client = FetchClient::new();
        let request = FetchRequestBuilder::new()
            .method("POST")
            .uri(Uri::from_static("http://localhost:3000"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::None)
            .redirect(RequestRedirect::Follow)
            .build()
            .unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
        let server = start_test_server(handler_redirect, rx, 3000);

        let response = client.execute(request).unwrap();
        let mut response_data: Option<FetchResponse> = None;
        tokio::select! {
            _ = server => {}
            response = response => {
                let response = response.unwrap();
                response_data = Some(response);
                tx.send(()).unwrap();
            }
        };

        assert!(response_data.is_some());
        let response = response_data.unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.status_message, "OK");
        assert_eq!(response.urls.len(), 3);
        assert_eq!(response.urls[0], "http://localhost:3000");
        assert_eq!(response.urls[1], "http://localhost:3000/1");
        assert_eq!(response.urls[2], "http://localhost:3000/2");
        assert_eq!(response.aborted, false);

        let body = match response.body {
            ResponseBody::DecodedStream(stream) => stream,
            _ => panic!("Expected body"),
        };
        let mut body = body.into_stream();
        let mut buffer = Vec::new();
        while let Some(chunk) = body.next().await {
            buffer.extend_from_slice(&chunk.unwrap());
        }
        let body = String::from_utf8(buffer).unwrap();
        assert!(body.contains("Hello, world!"));
    }

    #[tokio::test]
    async fn test_fetch_redirect_with_stream_body() {
        let client = FetchClient::new();
        let mut internal_stream = BoundedBufferChannel::new(10);
        internal_stream.try_write(vec![1, 2, 3, 4]).unwrap();

        let request = FetchRequestBuilder::new()
            .method("POST")
            .uri(Uri::from_static("http://localhost:3001"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::Stream(
                internal_stream
                    .aquire_reader()
                    .map(StreamDecoder::internal_stream),
            ))
            .redirect(RequestRedirect::Follow)
            .build()
            .unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
        let server = start_test_server(handler_redirect, rx, 3001);

        let response = client.execute(request).unwrap();
        let mut response_data: Option<FetchResponse> = None;
        tokio::select! {
            _ = server => {}
            response = response => {
                let response = response.unwrap();
                response_data = Some(response);
                tx.send(()).unwrap();
            }
        };

        assert!(response_data.is_some());
        let response = response_data.unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.status_message, "OK");
        assert_eq!(
            response.urls,
            vec![
                "http://localhost:3001",
                "http://localhost:3001/1",
                "http://localhost:3001/2"
            ]
        );
        assert_eq!(response.aborted, false);

        let body = match response.body {
            ResponseBody::DecodedStream(stream) => stream,
            _ => panic!("Expected body"),
        };
        let mut body = body.into_stream();
        let mut buffer = Vec::new();
        while let Some(chunk) = body.next().await {
            buffer.extend_from_slice(&chunk.unwrap());
        }
        let body = String::from_utf8(buffer).unwrap();
        assert!(body.contains("Hello, world!"));
    }

    #[tokio::test]
    async fn test_fetch_redirect_with_stream_error() {
        let client = FetchClient::new();
        let mut internal_stream = BoundedBufferChannel::new(10);
        internal_stream.try_write(vec![1, 2, 3, 4]).unwrap();

        let request = FetchRequestBuilder::new()
            .method("POST")
            .uri(Uri::from_static("http://localhost:3002"))
            .headers(HeadersMap::new(vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("X-Redirect-Status".to_string(), "302".to_string()),
            ]))
            .body(RequestBody::Stream(
                internal_stream
                    .aquire_reader()
                    .map(StreamDecoder::internal_stream),
            ))
            .redirect(RequestRedirect::Follow)
            .build()
            .unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
        let server = start_test_server(handler_redirect, rx, 3002);

        let response: PendingRequest = client.execute(request).unwrap();
        let mut err_data: Option<FetchError> = None;
        tokio::select! {
            _ = server => {}
            response = response => {
                assert!(response.is_err());
                if let Err(err) = response {
                    err_data = Some(err);
                }

                if let Err(err) = tx.send(()) {
                    eprintln!("Error sending signal: {}", err);
                }
            }
        };

        assert!(err_data.is_some());
        assert_eq!(
            format!("{}", err_data.unwrap()),
            "Redirect cannot be followed with stream body"
        );
    }

    #[tokio::test]
    async fn test_fetch_too_many_redirects() {
        let client = FetchClient::new();
        let request = FetchRequestBuilder::new()
            .method("POST")
            .uri(Uri::from_static("http://localhost:3003/many"))
            .headers(HeadersMap::new(vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("X-Redirect-Status".to_string(), "301".to_string()),
            ]))
            .body(RequestBody::None)
            .redirect(RequestRedirect::Follow)
            .build()
            .unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
        let server = start_test_server(handler_redirect, rx, 3003);

        let response: PendingRequest = client.execute(request).unwrap();
        let mut err_data: Option<FetchError> = None;
        tokio::select! {
            _ = server => {}
            response = response => {
                assert!(response.is_err());
                if let Err(err) = response {
                    err_data = Some(err);
                }

                if let Err(err) = tx.send(()) {
                    eprintln!("Error sending signal: {}", err);
                }
            }
        };

        assert!(err_data.is_some());
        assert_eq!(format!("{}", err_data.unwrap()), "Too many redirects");
    }

    async fn basic_auth_handler(
        req: Request<body::Incoming>,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        println!("URL> {:?}", req.uri().host());
        let auth = req.headers().get(AUTHORIZATION).unwrap();
        let auth = auth.to_str().unwrap();
        let status = if auth == "Basic dXNlcm5hbWU6cGFzc3dvcmQ=" {
            StatusCode::OK
        } else {
            StatusCode::UNAUTHORIZED
        };

        Ok(Response::builder()
            .status(status)
            .body(Full::from(Bytes::from_static(b"")))
            .unwrap())
    }

    #[tokio::test]
    async fn test_basic_auth() {
        let client = FetchClient::new();
        let request = FetchRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("http://username:password@localhost:3004"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::None)
            .build()
            .unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
        let server = start_test_server(basic_auth_handler, rx, 3004);
        let response = client.execute(request).unwrap();
        let mut response_data: Option<FetchResponse> = None;
        tokio::select! {
            _ = server => {}
            response = response => {
                let response = response.unwrap();
                response_data = Some(response);
                tx.send(()).unwrap();
            }
        };

        assert!(response_data.is_some());
        let response = response_data.unwrap();
        assert_eq!(response.status, 200);
    }

    #[tokio::test]
    async fn test_basic_auth_fail() {
        let client = FetchClient::new();
        let request = FetchRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("http://wrong:pass@localhost:3005"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::None)
            .build()
            .unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
        let server = start_test_server(basic_auth_handler, rx, 3005);
        let response = client.execute(request).unwrap();
        let mut response_data: Option<FetchResponse> = None;
        tokio::select! {
            _ = server => {}
            response = response => {
                let response = response.unwrap();
                response_data = Some(response);
                tx.send(()).unwrap();
            }
        };

        assert!(response_data.is_some());
        let response = response_data.unwrap();
        assert_eq!(response.status, 401);
        assert!(response.status_message.contains("Unauthorized"));
    }

    #[tokio::test]
    async fn test_fetch_remove_sensitive_headers() {
        let headers = HeadersMap::new(vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), "Bearer token".to_string()),
            ("Cookie".to_string(), "cookie".to_string()),
            (
                "Proxy-Authorization".to_string(),
                "Bearer token".to_string(),
            ),
        ]);

        let mut request = FetchRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("http://localhost:3006"))
            .headers(headers.clone())
            .body(RequestBody::None)
            .build()
            .unwrap();

        FetchClient::remove_sensitive_headers(
            &mut request,
            &Uri::from_static("http://localhost:3007"),
        );
        assert_eq!(request.headers.inner.len(), 1);
        assert_eq!(
            request.headers.inner[0],
            ("Content-Type".to_string(), "application/json".to_string())
        );
    }

    #[tokio::test]
    async fn test_fetch_decompression() {
        let client = FetchClient::new();
        let request = FetchRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("https://httpbin.org/gzip"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::None)
            .build()
            .unwrap();

        let response = client.execute(request).unwrap().await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.status_message, "OK");
        assert_eq!(response.urls[0], "https://httpbin.org/gzip");
        assert_eq!(response.aborted, false);

        let body = match response.body {
            ResponseBody::DecodedStream(stream) => stream,
            _ => panic!("Expected body"),
        };
        let mut body = body.into_stream();
        let mut buffer = Vec::new();
        while let Some(chunk) = body.next().await {
            buffer.extend_from_slice(&chunk.unwrap());
        }
        let body = String::from_utf8(buffer).unwrap();
        assert!(body.contains("gzipped"));
    }

    #[tokio::test]
    async fn test_fetch_network_error() {
        let client = FetchClient::new();
        let request = FetchRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("http://localhost:3008"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::None)
            .build()
            .unwrap();

        let response = client.execute(request).unwrap().await;
        assert!(response.is_err());
        let err = if let Err(err) = response {
            err
        } else {
            panic!("Expected error");
        };
        assert_eq!(err.message, "client error (Connect)");
    }
}
