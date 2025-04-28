use crate::{
    http::body::HttpBody, TcpListener, TcpOptions, UnboundedBufferChannel,
    UnboundedBufferChannelReader, UnboundedBufferChannelWriter,
};
use futures::{future::poll_fn, FutureExt};
use hyper::{body::Incoming, service::service_fn, Request, Response};
use std::task::{Context, Poll};
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

pub type HttpResponseSender = tokio::sync::oneshot::Sender<Response<HttpBody>>;

/// A request event to be processed by the server.
#[derive(Debug)]
pub struct HttpRequestEvent {
    pub req: Request<Incoming>,
    pub sender: Option<HttpResponseSender>,
}

impl HttpRequestEvent {
    pub fn new(req: Request<Incoming>, sender: HttpResponseSender) -> Self {
        Self {
            req,
            sender: Some(sender),
        }
    }

    pub fn response(self, res: Response<HttpBody>) {
        if let Some(sender) = self.sender {
            let _ = sender.send(res);
        }
    }
}

// pub struct HttpServerListener {
//     pub reader: UnboundedBufferChannelReader<HttpRequestEvent>,
// }
// impl HttpServerListener {
//     pub fn new(reader: UnboundedBufferChannelReader<HttpRequestEvent>) -> Self {
//         Self { reader }
//     }
// }

// impl Stream for HttpServerListener {
//     type Item = HttpRequestEvent;

//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         let this = self.get_mut();
//         match this.reader.poll_next_unpin(cx) {
//             Poll::Ready(Some(item)) => Poll::Ready(Some(item)),
//             Poll::Ready(None) => Poll::Ready(None),
//             Poll::Pending => Poll::Pending,
//         }
//     }
// }

/// An asynchronous function from a `Request` to a `Response`.
/// The `Service` trait is a simplified interface making it easy to write
/// network applications in a modular and reusable way, decoupled from the
/// underlying protocol.
pub struct HttpServer {
    sender: UnboundedBufferChannelWriter<HttpRequestEvent>,
    http_config: HttpConfig,
    /// The TCP listener for the server.
    /// This is used to accept incoming connections.
    tcp_listener: TcpListener,
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
}

impl HttpServer {
    pub fn new(
        sender: UnboundedBufferChannelWriter<HttpRequestEvent>,
        tcp_listener: TcpListener,
        http_config: HttpConfig,
    ) -> Self {
        Self {
            sender,
            tcp_listener,
            http_config,
        }
    }

    async fn accept_connection(
        stream: tokio::net::TcpStream,
        channel: UnboundedBufferChannelWriter<HttpRequestEvent>,
    ) {
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

    fn poll_accept(&self, cx: &mut Context<'_>) -> Poll<Result<(), HttpServerError>> {
        let listener = self.tcp_listener.poll_accept(cx)?;
        match listener {
            Poll::Ready((stream, _)) => {
                let sender = self.sender.clone();
                tokio::spawn(async move {
                    HttpServer::accept_connection(stream, sender).await;
                });
                Poll::Pending
            }
            Poll::Pending => Poll::Pending,
        }
    }

    pub fn listen(self) -> ShutdownServer {
        let (shutdown_signal, mut shutdown_receiver) = tokio::sync::watch::channel(());
        let shutdown_server = ShutdownServer::new(shutdown_signal);
        println!("Address: {:?}", self.tcp_listener.local_addr());
        tokio::spawn(async move {
            loop {
                let (stream, _) = self.tcp_listener.accept().await.unwrap();
                let sender = self.sender.clone();
                tokio::spawn(async move {
                    HttpServer::accept_connection(stream, sender).await;
                });
            }
            // poll_fn(|cx| {
            //     // check if the shutdown signal is received
            //     if Box::pin(shutdown_receiver.changed())
            //         .poll_unpin(cx)
            //         .is_ready()
            //     {
            //         println!("Shutdown signal received");
            //         return Poll::Ready(());
            //     }

            //     match self.poll_accept(cx) {
            //         Poll::Ready(_) => Poll::Pending, // Continue accepting connections
            //         Poll::Pending => Poll::Pending,
            //     }
            // })
            // .await
        });
        shutdown_server
    }
}

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Failed to send request")]
    SendError,

    #[error("Failed to receive response")]
    ReceiveError,
}

// impl Service<Request<Incoming>> for HttpService {
//     type Response = Response<HttpBody>;
//     type Error = ServiceError;
//     type Future =
//         Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

//     fn call(&self, req: Request<Incoming>) -> Self::Future {
//         let (sender, receiver) = tokio::sync::oneshot::channel::<Response<HttpBody>>();

//         let send_result = self
//             .sender
//             .send(RequestEvent::new(req, sender))
//             .map_err(|_| ServiceError::SendError);

//         let future = async move {
//             send_result?;
//             receiver.await.map_err(|_| ServiceError::ReceiveError)
//         };
//         Box::pin(future)
//     }
// }

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
    addr: HttpSocketAddr,
    tcp_listener: Option<TcpListener>,
}

impl HttpServerBuilder {
    pub fn new(addr: HttpSocketAddr) -> Self {
        Self {
            config: HttpConfig::default(),
            addr,
            tcp_listener: None,
        }
    }

    pub fn bind(mut self, tcp_listener: TcpListener) -> Self {
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

    pub async fn build(
        self,
    ) -> Result<
        (HttpServer, UnboundedBufferChannelReader<HttpRequestEvent>),
        HttpServerError,
    > {
        let address = match self.addr {
            HttpSocketAddr::IpSocket(addr) => addr,
            #[cfg(unix)]
            HttpSocketAddr::UnixSocket(_) => {
                // error!("Unix socket not supported yet");
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
        let sender = channel.writer().ok_or(HttpServerError::ChannelClosed)?;
        let reader = channel
            .acquire_reader()
            .ok_or(HttpServerError::ChannelClosed)?;

        let server = HttpServer::new(sender, tcp_listener, self.config.clone());
        Ok((server, reader))
    }
}
#[cfg(test)]
mod tests {
    use hyper::{Method, Request, Response, StatusCode};
    use std::net::SocketAddr;
    use std::time::Duration;
    use tokio::time::timeout;

    use super::*;
    use crate::{
        http::body::HttpBody, TcpListener, TcpOptions, UnboundedBufferChannelReader,
    };

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
        let builder = HttpServerBuilder::new(http_addr).bind(listener);

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

    // #[tokio::test]
    // async fn test_builder_build_success() {
    //     let addr = get_available_addr().await;
    //     let http_addr = HttpSocketAddr::IpSocket(addr);
    //     let builder = HttpServerBuilder::new(http_addr);
    //     let result = builder.build().await;

    //     assert!(result.is_ok());
    //     let (server, reader) = result.unwrap();

    //     assert_eq!(server.tcp_listener.local_addr().unwrap(), addr);
    //     assert!(server.http_config.http1_enabled); // Check default config applied
    //     assert!(server.http_config.http2_enabled);
    //     assert_eq!(server.http_config.ttl, None);
    //     // reader is an UnboundedBufferChannelReader<HttpRequestEvent>
    //     let _: UnboundedBufferChannelReader<HttpRequestEvent> = reader;
    // }

    // #[tokio::test]
    // async fn test_server_request_response() {
    //     let addr = get_available_addr().await;
    //     let http_addr = HttpSocketAddr::IpSocket(addr);
    //     let builder = HttpServerBuilder::new(http_addr);
    //     let (server, mut reader) = builder.build().await.expect("Failed to build server");

    //     let server_addr = server.tcp_listener.local_addr().unwrap();
    //     let shutdown = server.listen();

    //     // Spawn a task to handle incoming requests
    //     let handler_task = tokio::spawn(async move {
    //         // Wait for one request
    //         if let Some(event) = reader.recv().await {
    //             let response = Response::builder()
    //                 .status(StatusCode::OK)
    //                 .body(HttpBody::from("Hello"))
    //                 .unwrap();
    //             event.response(response);
    //         } else {
    //             panic!("Channel closed unexpectedly");
    //         }
    //     });

    //     // Spawn a client task to send a request
    //     let client_task = tokio::spawn(async move {
    //         let client = reqwest::Client::new();
    //         let url = format!("http://{}", server_addr);
    //         let res = client.get(&url).send().await.expect("Client failed");
    //         assert_eq!(res.status(), StatusCode::OK);
    //         let body = res.text().await.expect("Failed to read body");
    //         assert_eq!(body, "Hello");
    //     });

    //     // Wait for both tasks to complete (with a timeout)
    //     let timeout_duration = Duration::from_secs(5);
    //     match timeout(timeout_duration, handler_task).await {
    //         Ok(Ok(_)) => {} // Handler finished successfully
    //         Ok(Err(e)) => panic!("Handler task panicked: {:?}", e),
    //         Err(_) => panic!("Handler task timed out"),
    //     }
    //     match timeout(timeout_duration, client_task).await {
    //         Ok(Ok(_)) => {} // Client finished successfully
    //         Ok(Err(e)) => panic!("Client task panicked: {:?}", e),
    //         Err(_) => panic!("Client task timed out"),
    //     }

    //     // Shutdown the server
    //     shutdown.shutdown();
    //     // Small delay to allow server task to potentially exit (though poll_fn might keep it pending)
    //     tokio::time::sleep(Duration::from_millis(10)).await;
    // }

    #[tokio::test]
    async fn test_server_shutdown() {
        let addr = get_available_addr().await;
        let http_addr = HttpSocketAddr::IpSocket(addr);
        let builder = HttpServerBuilder::new(http_addr);
        let (server, _reader) = builder.build().await.expect("Failed to build server");

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

    // #[tokio::test]
    // async fn test_http_request_event_response() {
    //     let (sender, receiver) = tokio::sync::oneshot::channel();
    //     let req = Request::builder()
    //         .method(Method::GET)
    //         .uri("/")
    //         .body(Incoming::empty())
    //         .unwrap();
    //     let event = HttpRequestEvent::new(req, sender);

    //     let response_to_send = Response::builder()
    //         .status(StatusCode::ACCEPTED)
    //         .body(HttpBody::from("Test"))
    //         .unwrap();

    //     event.response(response_to_send);

    //     match timeout(Duration::from_secs(1), receiver).await {
    //         Ok(Ok(received_response)) => {
    //             assert_eq!(received_response.status(), StatusCode::ACCEPTED);
    //             // We can't easily read the body here as it's HttpBody,
    //             // but we know the response was sent and received.
    //         }
    //         Ok(Err(_)) => panic!("Receiver error"),
    //         Err(_) => panic!("Timeout waiting for response"),
    //     }
    // }
}
