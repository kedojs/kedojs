use futures::Stream;
// use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::net::{TcpListener as TokioTcpListener, TcpStream};

/// Options for configuring TCP sockets
#[derive(Debug, Clone)]
pub struct TcpOptions {
    ttl: Option<u32>,
    nodelay: bool,
    reuse_addr: bool,
    keepalive: bool,
}

impl TcpOptions {
    /// Create a new TcpOptions instance with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the Time-to-Live (TTL) for the socket
    pub fn ttl(mut self, ttl: u32) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Set whether to disable Nagle's algorithm
    pub fn nodelay(mut self, nodelay: bool) -> Self {
        self.nodelay = nodelay;
        self
    }

    /// Set whether to reuse the address
    pub fn reuse_addr(mut self, reuse_addr: bool) -> Self {
        self.reuse_addr = reuse_addr;
        self
    }

    /// Set whether to enable keepalive
    pub fn keepalive(mut self, keepalive: bool) -> Self {
        self.keepalive = keepalive;
        self
    }
}

impl Default for TcpOptions {
    fn default() -> Self {
        Self {
            ttl: None,
            nodelay: false,
            reuse_addr: true,
            keepalive: true,
        }
    }
}

/// A TCP socket server
#[derive(Debug)]
pub struct TcpListener {
    // socker: tokio::net::TcpSocket,
    listener: TokioTcpListener,
    options: TcpOptions,
}

impl TcpListener {
    /// Bind to a socket address with the given options
    pub async fn bind(addr: SocketAddr, options: TcpOptions) -> io::Result<Self> {
        let socket = if addr.is_ipv4() {
            tokio::net::TcpSocket::new_v4()?
        } else {
            tokio::net::TcpSocket::new_v6()?
        };

        socket.bind(addr)?;
        socket.set_reuseaddr(options.reuse_addr)?;
        socket.set_nodelay(options.nodelay)?;
        socket.set_keepalive(options.keepalive)?;

        let listener = socket.listen(10024)?;

        // Set the TTL if specified
        if let Some(ttl) = options.ttl {
            listener.set_ttl(ttl)?;
        }

        Ok(Self { listener, options })
    }

    pub fn poll_accept(
        &self,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<(TcpStream, SocketAddr)>> {
        self.listener.poll_accept(cx)
    }

    /// Accept a new connection
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let (stream, addr) = self.listener.accept().await?;

        // Apply connection-specific options
        // stream.set_nodelay(self.options.nodelay)?;

        // if let Some(ttl) = self.options.ttl {
        //     stream.set_ttl(ttl)?;
        // }

        Ok((stream, addr))
    }

    /// Get the local address this listener is bound to
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}

/// Implementation of Stream for TcpListener to support async iteration
impl Stream for TcpListener {
    type Item = io::Result<(TcpStream, SocketAddr)>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.listener.poll_accept(cx) {
            Poll::Ready(Ok((stream, addr))) => {
                // Apply connection-specific options
                if let Err(e) = stream.set_nodelay(this.options.nodelay) {
                    return Poll::Ready(Some(Err(e)));
                }

                Poll::Ready(Some(Ok((stream, addr))))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct TcpConnection {
    pub stream: TcpStream,
}

impl TcpConnection {
    /// Create a new TCP connection with the given options
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub async fn connect(addr: SocketAddr, options: TcpOptions) -> io::Result<Self> {
        let socket = if addr.is_ipv4() {
            tokio::net::TcpSocket::new_v4()?
        } else {
            tokio::net::TcpSocket::new_v6()?
        };

        socket.set_reuseaddr(options.reuse_addr)?;
        socket.set_nodelay(options.nodelay)?;
        socket.set_keepalive(options.keepalive)?;
        let stream = socket.connect(addr).await?;

        // Set the TTL if specified
        if let Some(ttl) = options.ttl {
            stream.set_ttl(ttl)?;
        }

        return Ok(Self { stream });
    }
}

// pub struct TcpConnManager {
//     listener: TcpListener,
//     shutdown_rx: tokio::sync::oneshot::Receiver<()>,
//     stream_tx: tokio::sync::mpsc::UnboundedSender<tokio::net::TcpStream>,
// }

// pub struct TcpConnManagerHandle {
//     shutdown_tx: tokio::sync::oneshot::Sender<()>,
//     stream_rx: tokio::sync::mpsc::UnboundedReceiver<tokio::net::TcpStream>,
// }

// pub struct TcpConnManagerBuilder {
//     listener: Option<TcpListener>,
//     shutdown_rx: Option<tokio::sync::oneshot::Receiver<()>>,
//     stream_tx: Option<tokio::sync::mpsc::UnboundedSender<tokio::net::TcpStream>>,
// }

// impl TcpConnManagerBuilder {
//     pub fn new() -> Self {
//         Self {
//             listener: None,
//             shutdown_rx: None,
//             stream_tx: None,
//         }
//     }

//     pub fn listener(mut self, listener: TcpListener) -> Self {
//         self.listener = Some(listener);
//         self
//     }

//     pub fn build(self) -> (TcpConnManager, TcpConnManagerHandle) {
//         let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
//         let (stream_tx, stream_rx) = tokio::sync::mpsc::unbounded_channel();
//         let handle = TcpConnManagerHandle {
//             shutdown_tx,
//             stream_rx,
//         };
//         let listener = self.listener.unwrap();
//         let manager = TcpConnManager {
//             listener,
//             shutdown_rx,
//             stream_tx,
//         };
//         (manager, handle)
//     }
// }

// impl TcpConnManagerHandle {
//     pub fn shutdown(self) {
//         let _ = self.shutdown_tx.send(());
//     }
// }

// impl TcpConnManagerHandle {
//     pub fn poll_recv(
//         &mut self,
//         cx: &mut Context<'_>,
//     ) -> Poll<Option<tokio::net::TcpStream>> {
//         self.stream_rx.poll_recv(cx)
//     }
// }

// impl Future for TcpConnManager {
//     type Output = io::Result<()>;

//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         let this = self.get_mut();

//         // Check if the shutdown signal has been received
//         if let Poll::Ready(Ok(())) = Pin::new(&mut this.shutdown_rx).poll(cx) {
//             return Poll::Ready(Ok(()));
//         }
//         // Poll the listener for incoming connections
//         loop {
//             match this.listener.poll_accept(cx) {
//                 Poll::Ready(Ok((stream, _))) => {
//                     if let Err(e) = this.stream_tx.send(stream) {
//                         return Poll::Ready(Err(io::Error::new(
//                             io::ErrorKind::Other,
//                             e.to_string(),
//                         )));
//                     }
//                 }
//                 Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
//                 Poll::Pending => return Poll::Pending,
//             }
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_tcp_listener_bind() {
        let addr = "127.0.0.1:0".parse().unwrap();
        let options = TcpOptions::default();
        let listener = TcpListener::bind(addr, options).await.unwrap();
        assert!(listener.local_addr().is_ok());
    }

    #[tokio::test]
    async fn test_tcp_options() {
        let options = TcpOptions::new()
            .ttl(128)
            .nodelay(false)
            .reuse_addr(true)
            .keepalive(false);

        assert_eq!(options.ttl, Some(128));
        assert_eq!(options.nodelay, false);
        assert_eq!(options.reuse_addr, true);
        assert_eq!(options.keepalive, false);
    }

    #[tokio::test]
    async fn test_tcp_connection() {
        let addr = "127.0.0.1:0".parse().unwrap();
        let options = TcpOptions::default();
        let listener = TcpListener::bind(addr, options.clone()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"ping");
            stream.write_all(b"pong").await.unwrap();
        });

        // Connect to the server
        let mut conn = TcpConnection::connect(server_addr, options).await.unwrap();
        conn.stream.write_all(b"ping").await.unwrap();

        let mut buf = [0u8; 4];
        conn.stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"pong");
    }

    // #[tokio::test]
    // async fn test_conn_manager() {
    //     // Create a listener
    //     let addr = "127.0.0.1:0".parse().unwrap();
    //     let options = TcpOptions::default();
    //     let listener = TcpListener::bind(addr, options).await.unwrap();
    //     let server_addr = listener.local_addr().unwrap();

    //     // Build the connection manager with the listener
    //     let (manager, mut handle) =
    //         TcpConnManagerBuilder::new().listener(listener).build();

    //     // Spawn the manager to handle connections
    //     let manager_handle = tokio::spawn(manager);

    //     // Create a task to handle incoming connections from the manager
    //     let connection_handler = tokio::spawn(async move {
    //         if let Some(mut stream) = handle.stream_rx.recv().await {
    //             let mut buf = [0u8; 4];
    //             stream.read_exact(&mut buf).await.unwrap();
    //             assert_eq!(&buf, b"ping");
    //             stream.write_all(b"pong").await.unwrap();
    //         }
    //     });

    //     // Connect to the server
    //     let mut conn = TcpConnection::connect(server_addr, TcpOptions::default())
    //         .await
    //         .unwrap();
    //     conn.stream.write_all(b"ping").await.unwrap();

    //     let mut buf = [0u8; 4];
    //     conn.stream.read_exact(&mut buf).await.unwrap();
    //     assert_eq!(&buf, b"pong");

    //     // Make sure all tasks are done
    //     connection_handler.await.unwrap();
    //     handle.shutdown_tx.send(()).unwrap();
    //     manager_handle.await.unwrap().unwrap();
    // }
}
