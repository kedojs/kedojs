use crate::http::errors::FetchError;
use crate::UnboundedBufferChannelReader;
use bytes::Bytes;
use futures::Stream;
use http_body_util::combinators::BoxBody;
use hyper::body::{Body, Incoming};
use std::pin::Pin;

#[derive(Debug)]
pub struct IncomingBodyStream(Incoming);

impl Stream for IncomingBodyStream {
    type Item = Result<Bytes, FetchError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = std::pin::Pin::into_inner(self);

        loop {
            break match Pin::new(&mut this.0).poll_frame(cx) {
                std::task::Poll::Ready(Some(Ok(chunk))) => {
                    if let Ok(data) = chunk.into_data() {
                        if !data.is_empty() {
                            break std::task::Poll::Ready(Some(Ok(data)));
                        }
                    }

                    continue;
                }
                std::task::Poll::Ready(Some(Err(e))) => {
                    std::task::Poll::Ready(Some(Err(FetchError::from(e))))
                }
                std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
                std::task::Poll::Pending => std::task::Poll::Pending,
            };
        }
    }
}

impl IncomingBodyStream {
    pub fn new(incoming: Incoming) -> Self {
        IncomingBodyStream(incoming)
    }

    #[cfg(test)]
    pub fn new_test() -> Self {
        unsafe {
            let incoming = std::mem::zeroed();
            IncomingBodyStream(incoming)
        }
    }
}

#[derive(Debug)]
pub struct InternalBodyStream(UnboundedBufferChannelReader<Vec<u8>>);

impl Stream for InternalBodyStream {
    type Item = Result<Bytes, FetchError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = std::pin::Pin::into_inner(self);

        loop {
            break match Pin::new(&mut this.0).poll_next(cx) {
                std::task::Poll::Ready(Some(chunk)) => {
                    if !chunk.is_empty() {
                        let data = Bytes::from(chunk);
                        break std::task::Poll::Ready(Some(Ok(data)));
                    }

                    continue;
                }
                std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
                std::task::Poll::Pending => std::task::Poll::Pending,
            };
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl InternalBodyStream {
    pub fn new(stream: UnboundedBufferChannelReader<Vec<u8>>) -> Self {
        InternalBodyStream(stream)
    }
}

impl Body for InternalBodyStream {
    type Data = bytes::Bytes;
    type Error = FetchError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>>
    {
        let this = std::pin::Pin::into_inner(self);

        loop {
            break match Pin::new(&mut this.0).poll_next(cx) {
                std::task::Poll::Ready(Some(chunk)) => {
                    if !chunk.is_empty() {
                        let data = Bytes::from(chunk);
                        break std::task::Poll::Ready(Some(Ok(
                            hyper::body::Frame::data(data),
                        )));
                    }

                    continue;
                }
                std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
                std::task::Poll::Pending => std::task::Poll::Pending,
            };
        }
    }
}

pub type HttpBody = BoxBody<bytes::Bytes, FetchError>;
