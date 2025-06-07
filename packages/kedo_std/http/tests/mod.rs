#[cfg(test)]
pub mod test_utils {
    use std::convert::Infallible;
    use std::error::Error;
    use std::future::Future;
    use std::net::SocketAddr;

    use bytes::Bytes;
    use http_body_util::Full;
    use hyper::Request;
    use hyper::{body, server::conn::http1, service::service_fn, Response};
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;
    use tokio::pin;

    pub async fn start_test_server<F>(
        handler: fn(Request<body::Incoming>) -> F,
        signal: tokio::sync::broadcast::Receiver<()>,
        server_port: u16,
    ) -> Result<(), Box<dyn Error>>
    where
        F: Future<Output = Result<Response<Full<Bytes>>, Infallible>> + Send + 'static,
    {
        let addr: SocketAddr = ([127, 0, 0, 1], server_port).into();

        let listener = TcpListener::bind(addr).await?;
        println!("Listening on http://{}", addr);

        loop {
            let (tcp, remote_address) = listener.accept().await?;
            let io = TokioIo::new(tcp);
            println!("Accepted connection from {:?}", remote_address);
            let mut signal_clone = signal.resubscribe();

            tokio::spawn(async move {
                let conn =
                    http1::Builder::new().serve_connection(io, service_fn(handler));
                pin!(conn);

                println!("Server connection established");
                tokio::select! {
                    res = conn.as_mut() => {
                        if let Err(e) = res {
                            eprintln!("server connection error: {}", e);
                        }
                    }
                    _ = signal_clone.recv() => {
                        conn.as_mut().graceful_shutdown();
                    }
                }
            });
        }

        Ok(())
    }
}
