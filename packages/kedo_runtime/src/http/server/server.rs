use std::{borrow::BorrowMut, future::Future, net::ToSocketAddrs, pin::Pin, sync::Arc};
use thiserror::Error;

use crate::http::{body::HttpBody, encoder::StreamEncoder};
use futures::{channel::oneshot, FutureExt as _, Stream};
use hyper::{body::Incoming, service::Service, Request, Response};
use tokio::{
    net::TcpListener,
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        watch,
    },
};
use tokio_rustls::rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig,
};

pub type RequestEventSender = oneshot::Sender<Response<HttpBody<StreamEncoder>>>;
/// A request event to be processed by the server.
/// The server will send a response back to the client using the `sender`.
/// The `req` field contains the incoming request.
pub struct RequestEvent {
    pub req: Request<Incoming>,
    // TODO: implement a custom struct to chain multiple oneshot signals
    // the main purpose is to close the connection when an error occurs or a shutdown signal is received
    pub sender: Option<RequestEventSender>,
}

// TODO: request event should implement method to close the connection
impl RequestEvent {
    pub fn new(req: Request<Incoming>, sender: RequestEventSender) -> Self {
        Self {
            req,
            sender: Some(sender),
        }
    }

    pub fn response(self, res: Response<HttpBody<StreamEncoder>>) {
        if let Some(sender) = self.sender {
            let _ = sender.send(res);
        }
    }
}

pub struct RequestReceiver {
    inner: mpsc::UnboundedReceiver<RequestEvent>,
}

impl Stream for RequestReceiver {
    type Item = RequestEvent;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.poll_recv(cx)
    }
}

pub struct CompletationFuture {
    receiver: oneshot::Receiver<()>,
}

impl Future for CompletationFuture {
    type Output = Result<(), oneshot::Canceled>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.receiver.poll_unpin(cx)
    }
}

pub struct ServerHandle {
    shutdown_signal: Option<watch::Sender<()>>,
    completion: Option<CompletationFuture>,
}

impl ServerHandle {
    fn new(
        shutdown_signal: watch::Sender<()>,
        completion: oneshot::Receiver<()>,
    ) -> Self {
        Self {
            shutdown_signal: Some(shutdown_signal),
            completion: Some(CompletationFuture {
                receiver: completion,
            }),
        }
    }

    pub fn shutdown(mut self) {
        if let Some(shutdown_signal) = self.shutdown_signal.take() {
            let _ = shutdown_signal.send(());
        }
    }

    pub fn completion(&mut self) -> Option<CompletationFuture> {
        self.completion.take()
    }
}

/// An asynchronous function from a `Request` to a `Response`.
/// The `Service` trait is a simplified interface making it easy to write
/// network applications in a modular and reusable way, decoupled from the
/// underlying protocol.
pub struct HttpService {
    sender: UnboundedSender<RequestEvent>,
}

impl HttpService {
    pub fn new() -> (Self, UnboundedReceiver<RequestEvent>) {
        let (sender, receiver) = mpsc::unbounded_channel::<RequestEvent>();

        (Self { sender }, receiver)
    }
}

impl Clone for HttpService {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Failed to send request")]
    SendError,

    #[error("Failed to receive response")]
    ReceiveError,
}

impl Service<Request<Incoming>> for HttpService {
    type Response = Response<HttpBody<StreamEncoder>>;
    type Error = ServiceError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let (sender, receiver) = oneshot::channel::<Response<HttpBody<StreamEncoder>>>();

        let send_result = self
            .sender
            .send(RequestEvent {
                req,
                sender: Some(sender),
            })
            .map_err(|_| ServiceError::SendError);

        Box::pin(async move {
            send_result?;
            receiver.await.map_err(|_| ServiceError::ReceiveError)
        })
    }
}

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

// type for (ServerHandle, CompletationFuture, RequestReceiver)
type ServerResult = (ServerHandle, RequestReceiver);

pub struct HttpServer {
    pub(crate) addr: HttpSocketAddr,
    pub(crate) enable_http2: bool,
    pub(crate) enable_http1: bool,
    pub(crate) tls_config: Option<Arc<ServerConfig>>,
    pub(crate) ttl: Option<u32>,
    tcp_listener: Option<TcpListener>,
    acceptor: Option<tokio_rustls::TlsAcceptor>,
}

impl HttpServer {
    /// Sets the address to bind the server.
    pub fn bind(mut self, addr: impl ToSocketAddrs) -> Result<Self, HttpServerError> {
        self.addr = addr
            .to_socket_addrs()
            .map_err(|_| {
                HttpServerError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid address",
                ))
            })?
            .next()
            .ok_or(HttpServerError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid address",
            )))?
            .into();
        Ok(self)
    }

    /// Enables HTTP/1 support.
    pub fn enable_http1(mut self) -> Self {
        self.enable_http1 = true;
        self
    }

    /// Enables HTTP/2 support.
    pub fn enable_http2(mut self) -> Self {
        self.enable_http2 = true;
        self
    }

    pub fn tls_config(
        mut self,
        certs: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
    ) -> Result<Self, HttpServerError> {
        let mut config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| HttpServerError::TlsError(e.to_string()))?;

        // Configure ALPN protocols for HTTP/1 and HTTP/2
        if self.enable_http2 {
            config.alpn_protocols.push(b"h2".to_vec());
        }
        if self.enable_http1 {
            config.alpn_protocols.push(b"http/1.1".to_vec());
        }

        self.tls_config = Some(Arc::new(config));
        Ok(self)
    }

    pub fn acceptor(mut self, acceptor: tokio_rustls::TlsAcceptor) -> Self {
        self.acceptor = Some(acceptor);
        self
    }

    async fn accept_https_connection(
        stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
        serv_clone: HttpService,
        ref mut shutdown_receiver: watch::Receiver<()>,
    ) {
        let stream = hyper_util::rt::TokioIo::new(Box::pin(stream));
        let executor = hyper_util::rt::TokioExecutor::new();
        let conn = hyper_util::server::conn::auto::Builder::new(executor)
            .serve_connection_with_upgrades(stream, serv_clone)
            .into_owned();
        tokio::pin!(conn);

        tokio::select! {
            connection = conn.as_mut() => {
                match connection {
                    Ok(_) => {},
                    Err(_) => {
                        // eprintln!("Error accepting connection: {:?}", e);
                    }
                }
            },
            _ = shutdown_receiver.changed() => {
                conn.as_mut().graceful_shutdown();
                return;
            }
        }
    }

    async fn accept_http_connection(
        stream: tokio::net::TcpStream,
        serv_clone: HttpService,
        ref mut shutdown_receiver: watch::Receiver<()>,
    ) {
        let stream = hyper_util::rt::TokioIo::new(Box::pin(stream));
        let rt = hyper_util::rt::TokioExecutor::new();
        let builder = hyper_util::server::conn::auto::Builder::new(rt);
        let conn = builder.serve_connection_with_upgrades(stream, serv_clone);
        tokio::pin!(conn);

        tokio::select! {
            connection = conn.as_mut() => {
                match connection {
                    Ok(_) => {},
                    Err(_) => {
                        // eprintln!("Error accepting connection: {:?}", e);
                    }
                }
            },
            _ = shutdown_receiver.changed() => {
                conn.as_mut().graceful_shutdown();
                return;
            }
        }
    }

    fn accept_connection(
        &self,
        stream: tokio::net::TcpStream,
        serv_clone: HttpService,
        shutdown_receiver: watch::Receiver<()>,
    ) {
        let acceptor = self
            .acceptor
            .as_ref()
            .and_then(|acceptor| Some(acceptor.clone()));

        tokio::spawn(async move {
            match acceptor {
                Some(acceptor) => {
                    // TODO: handle error
                    let stream = acceptor.accept(stream).await.unwrap();
                    HttpServer::accept_https_connection(
                        stream,
                        serv_clone,
                        shutdown_receiver,
                    )
                    .await;
                }
                None => {
                    HttpServer::accept_http_connection(
                        stream,
                        serv_clone,
                        shutdown_receiver,
                    )
                    .await;
                }
            };
        });
    }

    pub fn accept(mut self) -> Result<ServerResult, HttpServerError> {
        let (completion_sender, completion_receiver) = oneshot::channel::<()>();
        let (shutdown_signal, mut shutdown_receiver) = watch::channel(());
        let (service, req_receiver) = HttpService::new();

        let server = async move {
            loop {
                let serv_clone = service.clone();
                tokio::select! {
                    connection = self.tcp_listener.as_ref().expect("Invalid TPC listener").accept() => {
                        let (stream, _) = match connection {
                            Ok(connection) => connection,
                            Err(e) => {
                                eprintln!("Error accepting connection: {:?}", e);
                                continue;
                            }
                        };

                        self.accept_connection(stream, serv_clone, shutdown_receiver.clone());
                        continue;
                    }
                    _ = tokio::signal::ctrl_c() => {
                        println!("Received Ctrl-C signal to shutdown");
                        break;
                    }
                    _ = shutdown_receiver.borrow_mut().changed() => {
                        println!("Received shutdown signal");
                        break;
                    }
                }
            }

            let _ = self.tcp_listener.take();
            match completion_sender.send(()) {
                _ => {}
            };
            shutdown_receiver.mark_changed();
            let _ = match self.acceptor.take() {
                Some(acceptor) => acceptor,
                None => return,
            };
        };

        tokio::spawn(server);
        Ok((
            ServerHandle::new(shutdown_signal, completion_receiver),
            RequestReceiver {
                inner: req_receiver,
            },
        ))
    }
}

pub struct HttpServerBuilder {
    addr: Option<HttpSocketAddr>,
    tls_config: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
    ttl: Option<u32>,
}

impl HttpServerBuilder {
    pub fn new() -> Self {
        Self {
            addr: None,
            tls_config: None,
            ttl: None,
        }
    }

    pub fn addr(mut self, addr: HttpSocketAddr) -> Self {
        self.addr = Some(addr);
        self
    }

    pub fn tls_config(
        mut self,
        certs: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
    ) -> Self {
        self.tls_config = Some((certs, key));
        self
    }

    pub fn ttl(mut self, ttl: u32) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub async fn start(self) -> Result<HttpServer, HttpServerError> {
        let addr = self
            .addr
            .ok_or(HttpServerError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Address is required",
            )))?;

        let tpc_listener = TcpListener::bind(match addr {
            HttpSocketAddr::IpSocket(addr) => addr,
            #[cfg(unix)]
            HttpSocketAddr::UnixSocket(_) => {
                return Err(HttpServerError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Unix sockets are not supported",
                )));
            }
        })
        .await?;

        if let Some(ttl) = self.ttl {
            tpc_listener.set_ttl(ttl)?;
        }

        let acceptor = match self.tls_config {
            Some((certs, key)) => {
                let mut config = ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(certs, key)
                    .map_err(|e| HttpServerError::TlsError(e.to_string()))?;

                // Configure ALPN protocols for HTTP/1 and HTTP/2
                config.alpn_protocols.push(b"h2".to_vec());
                config.alpn_protocols.push(b"http/1.1".to_vec());
                Some(tokio_rustls::TlsAcceptor::from(Arc::new(config)))
            }
            None => None,
        };

        Ok(HttpServer {
            addr,
            enable_http2: true,
            enable_http1: true,
            tls_config: None,
            ttl: None,
            tcp_listener: Some(tpc_listener),
            acceptor,
        })
    }
}

#[cfg(test)]
mod tests {

    use bytes::Bytes;
    use futures::stream::TryStreamExt;
    use futures::StreamExt;
    use http_body_util::{BodyExt, Either, Full};
    use hyper::Uri;
    use tokio_rustls::rustls::pki_types::pem::PemObject;

    use crate::http::{
        fetch::fetch::FetchClient,
        headers::HeadersMap,
        request::{FetchRequest, FetchRequestBuilder, RequestBody},
        response::ResponseBody,
    };

    use super::*;
    use std::{net::SocketAddr, path::PathBuf};

    #[tokio::test]
    async fn test_http_server_shutdown() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let server = HttpServerBuilder::new()
            .addr(HttpSocketAddr::IpSocket(addr))
            .start()
            .await
            .unwrap();

        let (mut handler, _) = server.accept().unwrap();

        let completion = handler.completion().unwrap();
        tokio::join!(completion, async move {
            // wait for some seconds then shutdown the server
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            handler.shutdown();
        })
        .0
        .unwrap();
    }

    #[tokio::test]
    async fn test_http_server() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
        let request = FetchRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("http://127.0.0.1:8080"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::None)
            .build()
            .unwrap();

        let server = HttpServerBuilder::new()
            .addr(HttpSocketAddr::IpSocket(addr))
            .start()
            .await
            .unwrap();

        let (mut handler, mut receiver) = server.accept().unwrap();
        let completion = handler.completion().unwrap();
        tokio::join!(
            completion,
            async move {
                while let Some(event) = receiver.next().await {
                    let mut req_event = event;
                    let req = req_event.req;
                    let res = handle_request(req);
                    let _ = req_event.sender.take().unwrap().send(res);
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
        .0
        .unwrap();
    }

    fn handle_request(_: Request<Incoming>) -> Response<HttpBody<StreamEncoder>> {
        Response::new(Either::Right(
            Full::from(Bytes::from_static(b"Hello")).boxed(),
        ))
    }

    fn load_certs(path: &str) -> Vec<CertificateDer<'static>> {
        let certfile = PathBuf::from(path);
        let certs = CertificateDer::pem_file_iter(certfile)
            .expect("Failed to load certificates")
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        certs
    }

    fn load_private_key(path: &str) -> PrivateKeyDer<'static> {
        let keyfile = PathBuf::from(path);
        let key =
            PrivateKeyDer::from_pem_file(keyfile).expect("Failed to load private key");
        key
    }

    async fn send_client_request(request: FetchRequest) -> (String, u16) {
        let client = FetchClient::new();
        let response = match client.execute(request).unwrap().await {
            Ok(res) => res,
            Err(err) => {
                panic!("Failed to fetch: {}", err.describe());
            }
        };

        let body = match response.body {
            ResponseBody::DecodedStream(stream) => stream,
            _ => panic!("Expected body"),
        };
        let mut body = body.into_stream();
        let mut buffer = Vec::new();
        while let Some(chunk) = body.next().await {
            buffer.extend_from_slice(&chunk.unwrap());
        }
        return (String::from_utf8(buffer).unwrap(), response.status);
    }

    async fn create_https_server(addr: SocketAddr) -> HttpServer {
        let path_tls_folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("tls");
        let pem_file = path_tls_folder.join("selfsigned.crt");
        let key_file = path_tls_folder.join("private.key");
        let certs = load_certs(pem_file.to_str().unwrap());
        let key = load_private_key(key_file.to_str().unwrap());

        HttpServerBuilder::new()
            .addr(HttpSocketAddr::IpSocket(addr))
            .tls_config(certs, key)
            .start()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_https_server() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 8082));
        let request = FetchRequestBuilder::new()
            .method("GET")
            .uri(Uri::from_static("https://localhost:8082"))
            .headers(HeadersMap::new(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]))
            .body(RequestBody::None)
            .build()
            .unwrap();

        let server = create_https_server(addr).await;
        let (mut handler, mut receiver) = server.accept().unwrap();
        let completion = handler.completion().unwrap();

        tokio::join!(
            completion,
            async move {
                while let Some(event) = receiver.next().await {
                    let mut req_event = event;
                    let req = req_event.req;
                    let res = handle_request(req);
                    let _ = req_event.sender.take().unwrap().send(res);
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
        .0
        .unwrap();
    }

    #[tokio::test]
    async fn test_https_server_multiple_requests() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 8083));

        let server = create_https_server(addr).await;
        let (mut handler, mut receiver) = server.accept().unwrap();
        let completion = handler.completion().unwrap();

        tokio::join!(
            completion,
            async move {
                let counter = 0;
                while let Some(event) = receiver.next().await {
                    let mut req_event = event;
                    let req = req_event.req;
                    let res = handle_request(req);
                    req_event.sender.take().unwrap().send(res).unwrap();
                    if counter == 3 {
                        break;
                    }
                }
            },
            async move {
                for _ in 0..3 {
                    let request = FetchRequestBuilder::new()
                        .method("GET")
                        .uri(Uri::from_static("https://localhost:8083"))
                        .headers(HeadersMap::new(vec![(
                            "Content-Type".to_string(),
                            "application/json".to_string(),
                        )]))
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
        .0
        .unwrap();
    }
}
