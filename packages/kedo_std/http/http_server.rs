// use super::body::HttpBody;
// use crate::{
//     net::tcp::{TcpConnManager, TcpConnManagerHandle},
//     TcpListener,
// };
// use futures::{FutureExt as _, Stream, TryFutureExt};
// use hyper::{body::Incoming, service::Service, Request, Response};
// use std::{
//     cell::RefCell,
//     future::Future,
//     pin::Pin,
//     task::{Context, Poll},
// };
// use thiserror::Error;

// // pub struct ServerCompletion {
// //     inner: std::rc::Rc<RefCell<ServerCompletionInner>>,
// // }

// // #[derive(Debug, Default)]
// // pub struct ServerCompletionInner {
// //     closed: bool,
// //     waker: Option<std::task::Waker>,
// // }

// // impl ServerCompletion {
// //     pub fn send(&mut self) {
// //         let mut mut_ref = self.inner.borrow_mut();
// //         mut_ref.closed = true;
// //         if let Some(waker) = mut_ref.waker.take() {
// //             waker.wake();
// //         }
// //     }

// //     pub fn is_closed(&self) -> bool {
// //         self.inner.borrow_mut().closed
// //     }

// //     pub fn set_waker(&mut self, waker: std::task::Waker) {
// //         self.inner.borrow_mut().waker = Some(waker);
// //     }
// // }

// // impl Future for ServerCompletion {
// //     type Output = Result<(), ()>;

// //     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
// //         if self.is_closed() {
// //             Poll::Ready(Ok(()))
// //         } else {
// //             let self_mut = unsafe { self.get_unchecked_mut() };
// //             self_mut.set_waker(cx.waker().clone());
// //             Poll::Pending
// //         }
// //     }
// // }

// /// Error types for HTTP server operations
// #[derive(Error, Debug)]
// pub enum HttpServerError {
//     #[error("IO error occurred: {0}")]
//     IoError(#[from] std::io::Error),

//     #[error("TLS error occurred: {0}")]
//     TlsError(String),

//     #[error("Address already in use")]
//     AlreadyInUse,

//     #[error("Hyper error occurred: {0}")]
//     HyperError(#[from] hyper::Error),

//     #[error("Channel send error: {0}")]
//     SendError(String),

//     #[error("Channel receive error: {0}")]
//     ReceiveError(String),

//     #[error("Channel closed")]
//     ChannelClosed,
// }

// pub enum HttpSocketAddr {
//     IpSocket(std::net::SocketAddr),
//     #[cfg(unix)]
//     UnixSocket(tokio::net::unix::SocketAddr),
// }

// impl From<std::net::SocketAddr> for HttpSocketAddr {
//     fn from(addr: std::net::SocketAddr) -> Self {
//         Self::IpSocket(addr)
//     }
// }

// #[cfg(unix)]
// impl From<tokio::net::unix::SocketAddr> for HttpSocketAddr {
//     fn from(addr: tokio::net::unix::SocketAddr) -> Self {
//         Self::UnixSocket(addr)
//     }
// }

// pub type RequestEventSender = tokio::sync::oneshot::Sender<Response<HttpBody>>;

// /// A request event to be processed by the server.
// pub struct RequestEvent {
//     pub req: Request<Incoming>,
//     pub sender: Option<RequestEventSender>,
// }

// impl RequestEvent {
//     pub fn new(req: Request<Incoming>, sender: RequestEventSender) -> Self {
//         Self {
//             req,
//             sender: Some(sender),
//         }
//     }

//     pub fn response(self, res: Response<HttpBody>) {
//         if let Some(sender) = self.sender {
//             let _ = sender.send(res);
//         }
//     }
// }

// pub struct RequestReceiver {
//     inner: tokio::sync::mpsc::UnboundedReceiver<RequestEvent>,
// }

// impl Stream for RequestReceiver {
//     type Item = RequestEvent;

//     fn poll_next(
//         mut self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Option<Self::Item>> {
//         self.inner.poll_recv(cx)
//     }
// }

// impl RequestReceiver {
//     pub fn new(inner: tokio::sync::mpsc::UnboundedReceiver<RequestEvent>) -> Self {
//         Self { inner }
//     }

//     pub fn is_closed(&self) -> bool {
//         self.inner.is_closed()
//     }
// }

// pub struct CompletationFuture {
//     receiver: futures::channel::oneshot::Receiver<()>,
// }

// impl Future for CompletationFuture {
//     type Output = Result<(), futures::channel::oneshot::Canceled>;

//     fn poll(
//         mut self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Self::Output> {
//         self.receiver.poll_unpin(cx)
//     }
// }

// pub struct ServerHandle {
//     shutdown_signal: Option<futures::channel::oneshot::Sender<()>>,
//     receiver: Option<RequestReceiver>,
//     completion: Option<CompletationFuture>,
// }

// impl ServerHandle {
//     fn new(
//         shutdown_signal: futures::channel::oneshot::Sender<()>,
//         completion: futures::channel::oneshot::Receiver<()>,
//         receiver: RequestReceiver,
//     ) -> Self {
//         Self {
//             shutdown_signal: Some(shutdown_signal),
//             receiver: Some(receiver),
//             completion: Some(CompletationFuture {
//                 receiver: completion,
//             }),
//         }
//     }

//     pub fn shutdown(mut self) {
//         if let Some(shutdown_signal) = self.shutdown_signal.take() {
//             let _ = shutdown_signal.send(());
//         }
//     }

//     pub fn completion(&mut self) -> Option<CompletationFuture> {
//         self.completion.take()
//     }

//     pub fn receiver(&mut self) -> Option<RequestReceiver> {
//         self.receiver.take()
//     }
// }

// /// An asynchronous function from a `Request` to a `Response`.
// /// The `Service` trait is a simplified interface making it easy to write
// /// network applications in a modular and reusable way, decoupled from the
// /// underlying protocol.
// pub struct HttpService {
//     sender: tokio::sync::mpsc::UnboundedSender<RequestEvent>,
// }

// impl HttpService {
//     pub fn new() -> (Self, RequestReceiver) {
//         let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<RequestEvent>();

//         (Self { sender }, RequestReceiver::new(receiver))
//     }
// }

// impl Clone for HttpService {
//     fn clone(&self) -> Self {
//         Self {
//             sender: self.sender.clone(),
//         }
//     }
// }

// #[derive(Error, Debug)]
// pub enum ServiceError {
//     #[error("Failed to send request")]
//     SendError,

//     #[error("Failed to receive response")]
//     ReceiveError,
// }

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

// /// Configuration for HTTP server
// #[derive(Debug, Clone)]
// pub struct HttpConfig {
//     /// Enable HTTP/1.1 protocol
//     pub http1_enabled: bool,
//     /// Enable HTTP/2 protocol
//     pub http2_enabled: bool,
//     /// HTTP keep-alive timeout in seconds
//     pub ttl: Option<u32>,
// }

// impl Default for HttpConfig {
//     fn default() -> Self {
//         Self {
//             http1_enabled: true,
//             http2_enabled: true,
//             ttl: None,
//         }
//     }
// }

// // pub struct HttpAccepter {
// //     stream_rx: tokio::sync::mpsc::UnboundedReceiver<tokio::net::TcpStream>,
// //     service: HttpService,
// // }

// pub struct HttpConnManager {
//     tcp_conn: TcpConnManagerHandle,
//     stream_tx: tokio::sync::mpsc::UnboundedSender<tokio::net::TcpStream>,
// }

// impl Future for HttpConnManager {
//     type Output = Result<(), HttpServerError>;

//     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         // Poll the TCP listener
//         let conn = self.tcp_conn.poll_recv(cx);
//         match conn {
//             Poll::Ready(Some(stream)) => {
//                 self.stream_tx.send(stream).map_err(|_| {
//                     HttpServerError::SendError("Failed to send request".to_string())
//                 })?;
//             }
//             Poll::Ready(None) => {
//                 // Handle the case where the TCP listener is closed
//                 return Poll::Ready(Err(HttpServerError::ChannelClosed));
//             }
//             Poll::Pending => {}
//         };

//         Poll::Pending
//     }
// }

// pub struct HttpServerListener {
//     pub(crate) config: HttpConfig,
//     pub(crate) service: HttpService,
//     stream_rx: tokio::sync::mpsc::UnboundedReceiver<tokio::net::TcpStream>,
// }

// impl HttpServerListener {
//     pub fn new(
//         config: HttpConfig,
//         stream_rx: tokio::sync::mpsc::UnboundedReceiver<tokio::net::TcpStream>,
//         service: HttpService,
//     ) -> Self {
//         Self {
//             config,
//             stream_rx,
//             service,
//         }
//     }

//     pub async fn accept(&mut self) -> Result<(), HttpServerError> {
//         match self.stream_rx.recv().await {
//             Some(stream) => {
//                 let stream = hyper_util::rt::TokioIo::new(Box::pin(stream));
//                 let executor = hyper_util::rt::TokioExecutor::new();
//                 let conn = hyper_util::server::conn::auto::Builder::new(executor)
//                     .serve_connection_with_upgrades(stream, self.service.clone())
//                     .into_owned();
//                 match conn.await {
//                     Ok(_) => {
//                         // Handle the connection
//                         // ...
//                     }
//                     Err(e) => {
//                         println!("Error accepting connection: {:?}", e);
//                         // Handle the error
//                         // return Err(HttpServerError::HyperError(e));
//                     }
//                 }
//             }
//             None => return Err(HttpServerError::ChannelClosed),
//         };
//         Ok(())
//     }
// }

// impl Future for HttpServerListener {
//     type Output = Result<(), HttpServerError>;

//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         // Poll the TCP listener
//         let conn = self.stream_rx.poll_recv(cx);
//         match conn {
//             Poll::Ready(Some(stream)) => {
//                 let stream = hyper_util::rt::TokioIo::new(Box::pin(stream));
//                 let executor = hyper_util::rt::TokioExecutor::new();
//                 let conn = hyper_util::server::conn::auto::Builder::new(executor)
//                     .serve_connection_with_upgrades(stream, self.service.clone())
//                     .into_owned();
//                 match conn.await {
//                     Ok(_) => {
//                         // Handle the connection
//                         // ...
//                     }
//                     Err(e) => {
//                         println!("Error accepting connection: {:?}", e);
//                         // Handle the error
//                         // return Err(HttpServerError::HyperError(e));
//                     }
//                 }
//             }
//             Poll::Ready(None) => return Poll::Ready(Err(HttpServerError::ChannelClosed)),
//             Poll::Pending => {}
//         };

//         Poll::Pending
//     }
// }

// // pub struct HttpServer {
// //     pub(crate) config: HttpConfig,
// //     /// This is a TCP listener that listens for incoming connections
// //     listener: Option<TcpListener>,
// // }

// // pub struct TcpConnBuilder {
// //     enable_http2: bool,
// //     enable_http1: bool,
// //     listener: Option<TcpListener>,
// //     /// Time to live for the server in seconds
// //     ttl: Option<u32>,
// // }

// // impl TcpConnBuilder {
// //     pub fn new() -> Self {
// //         Self {
// //             ttl: None,
// //             enable_http2: true,
// //             enable_http1: true,
// //             listener: None,
// //         }
// //     }

// //     pub fn ttl(mut self, ttl: u32) -> Self {
// //         self.ttl = Some(ttl);
// //         self
// //     }

// //     pub fn enable_http2(mut self, enable: bool) -> Self {
// //         self.enable_http2 = enable;
// //         self
// //     }

// //     pub fn enable_http1(mut self, enable: bool) -> Self {
// //         self.enable_http1 = enable;
// //         self
// //     }

// //     pub fn listener(mut self, listener: TcpListener) -> Self {
// //         self.listener = Some(listener);
// //         self
// //     }

// //     pub fn build(self) -> Result<HttpServer, HttpServerError> {
// //         let listener = self.listener.take().ok_or(HttpServerError::AlreadyInUse)?;
// //         let (completion_tx, completion_rx) = futures::channel::oneshot::channel::<()>();
// //         let (shutdown_tx, shutdown_rx) = futures::channel::oneshot::channel::<()>();
// //         // let (service, req_rx) = HttpService::new();
// //         let server_handle = ServerHandle::new(shutdown_tx, completion_rx);

// //         Ok(HttpConnManager {
// //             // service,
// //             listener,
// //             shutdown_rx,
// //             completion_tx,
// //             config: self.config,
// //             // req_rx: Some(req_rx),
// //             server_handle: Some(server_handle),
// //             ctrl_c_future: Box::pin(tokio::signal::ctrl_c().map_err(|e| {
// //                 HttpServerError::IoError(std::io::Error::new(
// //                     std::io::ErrorKind::Other,
// //                     format!("Failed to listen for ctrl-c: {}", e),
// //                 ))
// //             })),
// //         })
// //         // let listener = self.listener;

// //         // Ok(HttpServer {
// //         //     config: HttpConfig {
// //         //         http1_enabled: self.enable_http1,
// //         //         http2_enabled: self.enable_http2,
// //         //         ttl: self.ttl,
// //         //     },
// //         //     listener,
// //         // })
// //     }
// // }

// // impl HttpServer {
// //     pub fn builder() -> HttpServerBuilder {
// //         HttpServerBuilder::new()
// //     }
// //     pub fn start(mut self) -> Result<HttpConnManager, HttpServerError> {
// //         let listener = self.listener.take().ok_or(HttpServerError::AlreadyInUse)?;
// //         let (completion_tx, completion_rx) = futures::channel::oneshot::channel::<()>();
// //         let (shutdown_tx, shutdown_rx) = futures::channel::oneshot::channel::<()>();
// //         // let (service, req_rx) = HttpService::new();
// //         let server_handle = ServerHandle::new(shutdown_tx, completion_rx);
// //         Ok(HttpConnManager {
// //             // service,
// //             listener,
// //             shutdown_rx,
// //             completion_tx,
// //             config: self.config,
// //             // req_rx: Some(req_rx),
// //             server_handle: Some(server_handle),
// //             ctrl_c_future: Box::pin(tokio::signal::ctrl_c().map_err(|e| {
// //                 HttpServerError::IoError(std::io::Error::new(
// //                     std::io::ErrorKind::Other,
// //                     format!("Failed to listen for ctrl-c: {}", e),
// //                 ))
// //             })),
// //         })
// //     }
// // }

// // impl Future for HttpServer {
// //     type Output = Result<(), HttpServerError>;
// //     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
// //         // if let Some(listener) = &self.listener {
// //         //     let mut listener = listener.clone();
// //         //     match listener.poll_accept(cx) {
// //         //         Poll::Ready(Ok(stream)) => {
// //         //             // Handle the stream
// //         //             // ...
// //         //             Poll::Ready(Ok(()))
// //         //         }
// //         //         Poll::Ready(Err(e)) => Poll::Ready(Err(HttpServerError::IoError(e))),
// //         //         Poll::Pending => Poll::Pending,
// //         //     }
// //         // } else {
// //         //     Poll::Ready(Ok(()))
// //         // }
// //         Poll::Ready(Ok(()))
// //     }
// // }
