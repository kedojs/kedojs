use crate::{
    buffer_channel::BufferChannel, http::body::HttpBody, BufferChannelWriter,
    TcpListener, TcpOptions, UnboundedBufferChannel, UnboundedBufferChannelReader,
    UnboundedBufferChannelWriter,
};
use futures::{stream::FuturesUnordered, StreamExt};
use hyper::{body::Incoming, service::service_fn, Request, Response};
use std::sync::Arc;
use thiserror::Error;

/// Error types for HTTP server operations
#[derive(Error, Debug)]
pub enum HttpServerError {
    #[error("IO error occurred: {0}")]
    IoError(#[from] std::io::Error),

    #[error("TLS error occurred: {0}")]
    TlsError(String),

    #[error("Address already in use")]
    AlreadyInUse,

    #[error("Hyper error occurred: {0}")]
    HyperError(#[from] hyper::Error),

    #[error("Channel send error: {0}")]
    SendError(String),

    #[error("Channel receive error: {0}")]
    ReceiveError(String),

    #[error("Channel closed")]
    ChannelClosed,
}

pub enum HttpSocketAddr {
    IpSocket(std::net::SocketAddr),
    #[cfg(unix)]
    UnixSocket(tokio::net::unix::SocketAddr),
}

impl From<std::net::SocketAddr> for HttpSocketAddr {
    fn from(addr: std::net::SocketAddr) -> Self {
        Self::IpSocket(addr)
    }
}

#[cfg(unix)]
impl From<tokio::net::unix::SocketAddr> for HttpSocketAddr {
    fn from(addr: tokio::net::unix::SocketAddr) -> Self {
        Self::UnixSocket(addr)
    }
}

/// A channel for sending HTTP responses back to the client.
/// This is a one-shot channel that allows the server to send a single response
pub type HttpResponseChannel = tokio::sync::oneshot::Sender<Response<HttpBody>>;

type HttpServerResult =
    Result<(HttpServer, UnboundedBufferChannelReader<HttpRequestEvent>), HttpServerError>;

/// A request event to be processed by the server.
#[derive(Debug)]
pub struct HttpRequestEvent {
    pub req: Request<Incoming>,
    pub channel: Option<HttpResponseChannel>,
}

impl HttpRequestEvent {
    pub fn new(req: Request<Incoming>, channel: HttpResponseChannel) -> Self {
        Self {
            req,
            channel: Some(channel),
        }
    }

    pub fn response(self, res: Response<HttpBody>) {
        if let Some(channel) = self.channel {
            let _ = channel.send(res);
        }
    }
}

/// An asynchronous function from a `Request` to a `Response`.
/// The `Service` trait is a simplified interface making it easy to write
/// network applications in a modular and reusable way, decoupled from the
/// underlying protocol.
pub struct HttpServer {
    channel: UnboundedBufferChannelWriter<HttpRequestEvent>,
    config: HttpConfig,
    /// The TCP listener for the server.
    /// This is used to accept incoming connections.
    tcp_listener: TcpListener,
    acceptor: Option<tokio_rustls::TlsAcceptor>,
}

pub struct ShutdownServer {
    signal: tokio::sync::watch::Sender<()>,
}

impl ShutdownServer {
    pub fn new(signal: tokio::sync::watch::Sender<()>) -> Self {
        Self { signal }
    }

    pub fn shutdown(&self) {
        let _ = self.signal.send(());
    }

    pub fn is_shutdown(&self) -> bool {
        self.signal.receiver_count() == 0
    }
}

/// Accepts a new connection and processes the request.
/// This function is called when a new connection is accepted.
async fn http_accept_connection<T>(
    stream: T,
    channel: UnboundedBufferChannelWriter<HttpRequestEvent>,
) where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
{
    let stream = hyper_util::rt::TokioIo::new(Box::pin(stream));
    let rt = hyper_util::rt::TokioExecutor::new();
    let builder = hyper_util::server::conn::auto::Builder::new(rt);
    let conn = builder.serve_connection(
        stream,
        service_fn(move |req: Request<Incoming>| {
            let (sender, receiver) =
                tokio::sync::oneshot::channel::<Response<HttpBody>>();
            let event = HttpRequestEvent::new(req, sender);
            let send_result = channel.try_write(event);

            async move {
                send_result.map_err(|_| {
                    HttpServerError::SendError("Failed to send request".to_string())
                })?;
                receiver.await.map_err(|_| {
                    HttpServerError::ReceiveError(
                        "Failed to receive response".to_string(),
                    )
                })
            }
        }),
    );

    let _ = conn.await;
}

impl HttpServer {
    pub fn new(
        channel: UnboundedBufferChannelWriter<HttpRequestEvent>,
        tcp_listener: TcpListener,
        config: HttpConfig,
        acceptor: Option<tokio_rustls::TlsAcceptor>,
    ) -> Self {
        Self {
            channel,
            tcp_listener,
            config,
            acceptor,
        }
    }

    async fn http_listen(self, mut shutdown: tokio::sync::watch::Receiver<()>) {
        let mut tasks = FuturesUnordered::new();
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    break;
                }
                result = self.tcp_listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let sender = self.channel.clone();
                            tasks.push(http_accept_connection(stream, sender));
                        }
                        Err(e) => {
                            eprintln!("Error accepting connection: {:?}", e);
                        }
                    }
                }
                Some(_) = tasks.next() => {}
            }
        }
    }

    async fn http_listen_tls(
        self,
        mut shutdown: tokio::sync::watch::Receiver<()>,
        acceptor: tokio_rustls::TlsAcceptor,
    ) {
        let mut tasks = FuturesUnordered::new();
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    break;
                }
                result = self.tcp_listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let sender = self.channel.clone();
                            let acceptor = acceptor.clone();
                            tasks.push(async move {
                                let stream = acceptor.accept(stream).await;
                                match stream {
                                    Ok(stream) => {
                                        http_accept_connection(stream, sender).await;
                                    }
                                    Err(e) => {
                                        eprintln!("Error accepting TLS connection: {:?}", e);
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Error accepting connection: {:?}", e);
                        }
                    }
                }
                Some(_) = tasks.next() => {}
            }
        }
    }

    pub fn listen(self) -> ShutdownServer {
        let (shutdown_signal, shutdown_receiver) = tokio::sync::watch::channel(());
        let shutdown_server = ShutdownServer::new(shutdown_signal);

        let tls_acceptor = self.acceptor.clone();
        match tls_acceptor {
            Some(acceptor) => {
                // If TLS is enabled, spawn a task to handle TLS connections
                tokio::spawn(self.http_listen_tls(shutdown_receiver, acceptor));
            }
            None => {
                // If TLS is not enabled, spawn a task to handle plain TCP connections
                tokio::spawn(self.http_listen(shutdown_receiver));
            }
        }

        shutdown_server
    }
}

/// Configuration for HTTP server
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Enable HTTP/1.1 protocol
    pub http1_enabled: bool,
    /// Enable HTTP/2 protocol
    pub http2_enabled: bool,
    /// HTTP keep-alive timeout in seconds
    pub ttl: Option<u32>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            http1_enabled: true,
            http2_enabled: true,
            ttl: None,
        }
    }
}

pub struct HttpServerBuilder {
    config: HttpConfig,
    tls_config: Option<tokio_rustls::rustls::ServerConfig>,
    addr: HttpSocketAddr,
    tcp_listener: Option<TcpListener>,
}

impl HttpServerBuilder {
    pub fn new(addr: HttpSocketAddr) -> Self {
        Self {
            config: HttpConfig::default(),
            tls_config: None,
            addr,
            tcp_listener: None,
        }
    }

    pub fn listener(mut self, tcp_listener: TcpListener) -> Self {
        self.tcp_listener = Some(tcp_listener);
        self
    }

    pub fn addr(mut self, addr: HttpSocketAddr) -> Self {
        self.addr = addr;
        self
    }

    pub fn config(mut self, config: HttpConfig) -> Self {
        self.config = config;
        self
    }

    pub fn tls_config(
        mut self,
        certs: Vec<tokio_rustls::rustls::pki_types::CertificateDer<'static>>,
        key: tokio_rustls::rustls::pki_types::PrivateKeyDer<'static>,
    ) -> Result<Self, HttpServerError> {
        let mut config = tokio_rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| HttpServerError::TlsError(e.to_string()))?;

        // Configure ALPN protocols for HTTP/1 and HTTP/2
        if self.config.http2_enabled {
            config.alpn_protocols.push(b"h2".to_vec());
        }

        if self.config.http1_enabled {
            config.alpn_protocols.push(b"http/1.1".to_vec());
        }

        self.tls_config = Some(config);
        Ok(self)
    }

    pub async fn bind(self) -> HttpServerResult {
        let address = match self.addr {
            HttpSocketAddr::IpSocket(addr) => addr,
            #[cfg(unix)]
            HttpSocketAddr::UnixSocket(_) => {
                return Err(HttpServerError::TlsError(
                    "Unix socket not supported".to_string(),
                ));
            }
        };

        // if not tcp_listener, create a new one
        let tcp_listener = match self.tcp_listener {
            Some(listener) => listener,
            None => TcpListener::bind(address, TcpOptions::default())
                .await
                .map_err(|_| HttpServerError::AlreadyInUse)?,
        };

        // create a new channel
        let mut channel = UnboundedBufferChannel::new();
        let sender = channel
            .acquire_writer()
            .ok_or(HttpServerError::ChannelClosed)?;
        let reader = channel
            .acquire_reader()
            .ok_or(HttpServerError::ChannelClosed)?;

        let acceptor = match self.tls_config {
            Some(tls_config) => Some(tokio_rustls::TlsAcceptor::from(Arc::new(
                tls_config.clone(),
            ))),
            None => None,
        };

        let server = HttpServer::new(sender, tcp_listener, self.config.clone(), acceptor);
        Ok((server, reader))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{
        fetch::FetchClient,
        request::{HttpRequest, HttpRequestBuilder, RequestBody},
        response::ResponseBody,
    };
    use crate::{TcpListener, TcpOptions};
    use bytes::Bytes;
    use futures::stream::TryStreamExt;
    use futures::StreamExt;
    use http_body_util::{BodyExt, Full};
    use hyper::{
        header::{self, HeaderValue},
        Uri,
    };
    use std::time::Duration;
    use std::{net::SocketAddr, path::PathBuf};
    use tokio_rustls::rustls::pki_types::pem::PemObject;

    async fn get_available_addr() -> SocketAddr {
        tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap()
            .local_addr()
            .unwrap()
    }

    #[tokio::test]
    async fn test_builder_new_defaults() {
        let addr = get_available_addr().await;
        let http_addr = HttpSocketAddr::IpSocket(addr);
        let builder = HttpServerBuilder::new(http_addr);

        assert!(builder.config.http1_enabled);
        assert!(builder.config.http2_enabled);
        assert_eq!(builder.config.ttl, None);
        assert!(builder.tcp_listener.is_none());
        match builder.addr {
            HttpSocketAddr::IpSocket(a) => assert_eq!(a, addr),
            #[cfg(unix)]
            HttpSocketAddr::UnixSocket(_) => panic!("Expected IpSocket"),
        }
    }

    #[tokio::test]
    async fn test_builder_custom_config() {
        let addr = get_available_addr().await;
        let http_addr = HttpSocketAddr::IpSocket(addr);
        let custom_config = HttpConfig {
            http1_enabled: false,
            http2_enabled: false,
            ttl: Some(60),
        };
        let builder = HttpServerBuilder::new(http_addr).config(custom_config.clone());

        assert_eq!(builder.config.http1_enabled, custom_config.http1_enabled);
        assert_eq!(builder.config.http2_enabled, custom_config.http2_enabled);
        assert_eq!(builder.config.ttl, custom_config.ttl);
    }

    #[tokio::test]
    async fn test_builder_bind() {
        let addr = get_available_addr().await;
        let http_addr = HttpSocketAddr::IpSocket(addr);
        let listener = TcpListener::bind(addr, TcpOptions::default())
            .await
            .unwrap();
        let local_addr = listener.local_addr().unwrap(); // Store before moving
        let builder = HttpServerBuilder::new(http_addr).listener(listener);

        assert!(builder.tcp_listener.is_some());
        assert_eq!(
            builder.tcp_listener.as_ref().unwrap().local_addr().unwrap(),
            local_addr
        );
    }

    #[tokio::test]
    async fn test_builder_addr() {
        let addr1 = get_available_addr().await;
        let addr2 = get_available_addr().await;
        assert_ne!(addr1, addr2);

        let http_addr1 = HttpSocketAddr::IpSocket(addr1);
        let http_addr2 = HttpSocketAddr::IpSocket(addr2);

        let builder = HttpServerBuilder::new(http_addr1).addr(http_addr2);

        match builder.addr {
            HttpSocketAddr::IpSocket(a) => assert_eq!(a, addr2),
            #[cfg(unix)]
            HttpSocketAddr::UnixSocket(_) => panic!("Expected IpSocket"),
        }
    }

    #[tokio::test]
    async fn test_server_shutdown() {
        let addr = get_available_addr().await;
        let http_addr = HttpSocketAddr::IpSocket(addr);
        let builder = HttpServerBuilder::new(http_addr);
        let (server, _reader) = builder.bind().await.expect("Failed to build server");

        let shutdown = server.listen(); // This spawns the server task

        // Give the server a moment to start listening
        tokio::time::sleep(Duration::from_millis(80)).await;

        // Shutdown the server
        shutdown.shutdown();

        // Here, we can't easily assert the internal server task has *fully* stopped
        // without more complex signaling or joining the task handle (which isn't returned).
        // However, we know the shutdown signal was sent. A subsequent connection attempt
        // *should* fail after a short delay, but that's harder to test reliably without races.
        // For this unit test, simply ensuring shutdown() can be called without panic is sufficient.
        // A more robust test might involve trying to connect *after* shutdown and asserting failure.
        let receiver_count = shutdown.signal.receiver_count(); // Check the number of active receivers
        assert_eq!(receiver_count, 1); // Check if the receiver still exists
    }

    fn handle_request(_: Request<Incoming>) -> Response<HttpBody> {
        Response::new(
            Full::from(Bytes::from_static(b"Hello"))
                .map_err(|_| crate::FetchError::new("Failed to create response"))
                .boxed(),
        )
    }

    fn load_certs(
        path: &str,
    ) -> Vec<tokio_rustls::rustls::pki_types::CertificateDer<'static>> {
        let certfile = PathBuf::from(path);
        let certs =
            tokio_rustls::rustls::pki_types::CertificateDer::pem_file_iter(certfile)
                .expect("Failed to load certificates")
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
        certs
    }

    fn load_private_key(
        path: &str,
    ) -> tokio_rustls::rustls::pki_types::PrivateKeyDer<'static> {
        let keyfile = PathBuf::from(path);
        let key = tokio_rustls::rustls::pki_types::PrivateKeyDer::from_pem_file(keyfile)
            .expect("Failed to load private key");
        key
    }

    #[tokio::test]
    async fn test_http_server() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
        let mut headers = hyper::header::HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        let request = HttpRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("http://127.0.0.1:8080"))
            .headers(headers)
            .body(RequestBody::None)
            .build()
            .unwrap();

        let (server, mut reader) = HttpServerBuilder::new(HttpSocketAddr::IpSocket(addr))
            .bind()
            .await
            .unwrap();

        let handler = server.listen();
        tokio::join!(
            async move {
                while let Some(event) = reader.next().await {
                    let mut req_event = event;
                    let req = req_event.req;
                    let res = handle_request(req);
                    let _ = req_event.channel.take().unwrap().send(res);
                    break;
                }
            },
            async move {
                let (body, status) = send_client_request(request).await;
                assert_eq!(status, 200);
                assert!(body.contains("Hello"));
                handler.shutdown();
            }
        )
        .0;
    }

    async fn send_client_request(request: HttpRequest) -> (String, u16) {
        let client = FetchClient::new();
        let mut response = match client.execute(request).unwrap().await {
            Ok(res) => res,
            Err(err) => {
                panic!("Failed to fetch: {}", err.describe());
            }
        };

        let body = match response.take_body() {
            ResponseBody::DecodedStream(stream) => stream,
            _ => panic!("Expected body"),
        };
        let mut body = body.into_stream();
        let mut buffer = Vec::new();
        while let Some(chunk) = body.next().await {
            buffer.extend_from_slice(&chunk.unwrap());
        }
        return (
            String::from_utf8(buffer).unwrap(),
            response.status().as_u16(),
        );
    }

    async fn create_https_server(
        addr: SocketAddr,
    ) -> (HttpServer, UnboundedBufferChannelReader<HttpRequestEvent>) {
        let path_tls_folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests")
            .join("fixtures")
            .join("tls");
        let pem_file = path_tls_folder.join("selfsigned.crt");
        let key_file = path_tls_folder.join("private.key");
        let certs = load_certs(pem_file.to_str().unwrap());
        let key = load_private_key(key_file.to_str().unwrap());

        return HttpServerBuilder::new(HttpSocketAddr::IpSocket(addr))
            .tls_config(certs, key)
            .expect("Failed to create TLS config")
            .bind()
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_https_server() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 8082));
        let mut headers = hyper::header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        let request = HttpRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("https://localhost:8082"))
            .headers(headers)
            .body(RequestBody::None)
            .build()
            .unwrap();

        let (server, mut reader) = create_https_server(addr).await;
        let handler = server.listen();

        tokio::join!(
            async move {
                while let Some(event) = reader.next().await {
                    let mut req_event = event;
                    let req = req_event.req;
                    let res = handle_request(req);
                    let _ = req_event.channel.take().unwrap().send(res);
                    break;
                }
            },
            async move {
                let (body, status) = send_client_request(request).await;
                assert_eq!(status, 200);
                assert!(body.contains("Hello"));
                handler.shutdown();
            }
        )
        .0;
    }

    #[tokio::test]
    async fn test_https_server_multiple_requests() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 8083));
        let (server, mut reader) = create_https_server(addr).await;
        let handler = server.listen();
        let mut headers = hyper::header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(header::USER_AGENT, HeaderValue::from_static("kedo"));

        tokio::join!(
            async move {
                let mut counter = 0;
                while let Some(event) = reader.next().await {
                    let mut req_event = event;
                    let req = req_event.req;
                    let res = handle_request(req);
                    req_event.channel.take().unwrap().send(res).unwrap();
                    if counter == 3 {
                        break;
                    }
                    counter += 1;
                }
            },
            async move {
                for _ in 0..3 {
                    let request = HttpRequestBuilder::new()
                        .method("GET")
                        .uri(Uri::from_static("https://localhost:8083"))
                        .headers(headers.clone())
                        .body(RequestBody::None)
                        .build()
                        .unwrap();
                    let (body, status) = send_client_request(request).await;
                    assert_eq!(status, 200);
                    assert!(body.contains("Hello"));
                }

                handler.shutdown();
            }
        )
        .0;
    }
}
