use std::{convert::Infallible, pin::Pin};

use bytes::Bytes;
use futures::Stream;
use http_body_util::{combinators::BoxBody, Either};
use hyper::body::{Body, Incoming};

use crate::streams::streams::InternalStreamResourceReader;

use super::fetch::errors::FetchError;

#[derive(Debug)]
pub struct HtppBodyStream<S>
where
    S: Stream<Item = Result<Bytes, FetchError>> + Unpin,
{
    inner: S,
}

impl<S> HtppBodyStream<S>
where
    S: Stream<Item = Result<Bytes, FetchError>> + Unpin,
{
    pub fn new(stream: S) -> Self {
        Self { inner: stream }
    }
}

impl<S> Stream for HtppBodyStream<S>
where
    S: Stream<Item = Result<Bytes, FetchError>> + Unpin,
{
    type Item = Result<Bytes, FetchError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.inner).poll_next(cx)
    }
}

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
}

#[derive(Debug)]
pub struct InternalBodyStream(InternalStreamResourceReader<Vec<u8>>);

impl Stream for InternalBodyStream {
    type Item = Result<Bytes, FetchError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = std::pin::Pin::into_inner(self);

        loop {
            break match Pin::new(&mut this.0).poll_next(cx) {
                std::task::Poll::Ready(Some(Ok(chunk))) => {
                    if !chunk.is_empty() {
                        let data = Bytes::from(chunk);
                        break std::task::Poll::Ready(Some(Ok(data)));
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

impl InternalBodyStream {
    pub fn new(stream: InternalStreamResourceReader<Vec<u8>>) -> Self {
        InternalBodyStream(stream)
    }
}

pub type HttpBody<T> = Either<T, BoxBody<bytes::Bytes, Infallible>>;
