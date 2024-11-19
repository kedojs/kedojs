use std::pin::Pin;

use crate::http::fetch_errors::FetchError;
use async_compression::tokio::bufread::BrotliDecoder;
use async_compression::tokio::bufread::GzipDecoder;
use async_compression::tokio::bufread::ZlibDecoder;
use async_compression::tokio::bufread::ZstdDecoder;

use bytes::Bytes;
use futures::ready;
use futures::Stream;
use hyper::body::SizeHint;
use hyper::body::{Body, Incoming};
use hyper::header::CONTENT_ENCODING;
use hyper::header::CONTENT_LENGTH;
use hyper::header::TRANSFER_ENCODING;
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_util::io::StreamReader;

use super::HeadersMap;

#[derive(Debug)]
pub struct ResponseStream(Incoming);

impl Stream for ResponseStream {
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

impl ResponseStream {
    pub fn new(incoming: Incoming) -> Self {
        ResponseStream(incoming)
    }
}

enum DecoderType {
    Gzip,
    Brotli,
    Zstd,
    Deflate,
}

impl TryFrom<&str> for DecoderType {
    type Error = FetchError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "gzip" => Ok(Self::Gzip),
            "br" => Ok(Self::Brotli),
            "zstd" => Ok(Self::Zstd),
            "deflate" => Ok(Self::Deflate),
            _ => Err(FetchError::new("Unsupported encoding")),
        }
    }
}

pub(crate) struct Decoder {
    inner: Inner,
}

impl Decoder {
    pub fn gzip(stream: ResponseStream) -> Self {
        Self {
            inner: Inner::Gzip(Box::pin(FramedRead::new(
                GzipDecoder::new(StreamReader::new(stream)),
                BytesCodec::new(),
            ))),
        }
    }

    pub fn brotli(stream: ResponseStream) -> Self {
        Self {
            inner: Inner::Brotli(Box::pin(FramedRead::new(
                BrotliDecoder::new(StreamReader::new(stream)),
                BytesCodec::new(),
            ))),
        }
    }

    pub fn zstd(stream: ResponseStream) -> Self {
        Self {
            inner: Inner::Zstd(Box::pin(FramedRead::new(
                ZstdDecoder::new(StreamReader::new(stream)),
                BytesCodec::new(),
            ))),
        }
    }

    pub fn deflate(stream: ResponseStream) -> Self {
        Self {
            inner: Inner::Deflate(Box::pin(FramedRead::new(
                ZlibDecoder::new(StreamReader::new(stream)),
                BytesCodec::new(),
            ))),
        }
    }

    pub fn plain(stream: ResponseStream) -> Self {
        Self {
            inner: Inner::Plain(stream),
        }
    }

    fn detect_encoding(headers: &HeadersMap, encoding_str: &str) -> bool {
        let mut is_content_encoded = {
            headers
                .get_all(CONTENT_ENCODING.as_str())
                .iter()
                .any(|enc| *enc == encoding_str)
                || headers
                    .get_all(TRANSFER_ENCODING.as_str())
                    .iter()
                    .any(|enc| *enc == encoding_str)
        };

        if is_content_encoded {
            if let Some(content_length) = headers.get(CONTENT_LENGTH.as_str()) {
                if content_length == "0" {
                    is_content_encoded = false;
                }
            }
        }

        is_content_encoded
    }

    pub fn detect(stream: ResponseStream, headers: &HeadersMap) -> Self {
        if Self::detect_encoding(headers, "gzip") {
            Self::gzip(stream)
        } else if Self::detect_encoding(headers, "br") {
            Self::brotli(stream)
        } else if Self::detect_encoding(headers, "zstd") {
            Self::zstd(stream)
        } else if Self::detect_encoding(headers, "deflate") {
            Self::deflate(stream)
        } else {
            Self::plain(stream)
        }
    }
}

type ResponseStreamReader = StreamReader<ResponseStream, Bytes>;

enum Inner {
    Gzip(Pin<Box<FramedRead<GzipDecoder<ResponseStreamReader>, BytesCodec>>>),
    Brotli(Pin<Box<FramedRead<BrotliDecoder<ResponseStreamReader>, BytesCodec>>>),
    Zstd(Pin<Box<FramedRead<ZstdDecoder<ResponseStreamReader>, BytesCodec>>>),
    Deflate(Pin<Box<FramedRead<ZlibDecoder<ResponseStreamReader>, BytesCodec>>>),
    Plain(ResponseStream),
}

impl Stream for Decoder {
    type Item = Result<Bytes, FetchError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = std::pin::Pin::into_inner(self);

        loop {
            break match this.inner {
                Inner::Gzip(ref mut decoder) => {
                    match ready!(Pin::new(decoder).poll_next(cx)) {
                        Some(Ok(chunk)) => {
                            std::task::Poll::Ready(Some(Ok(chunk.freeze())))
                        }
                        Some(Err(e)) => {
                            std::task::Poll::Ready(Some(Err(FetchError::from(e))))
                        }
                        None => std::task::Poll::Ready(None),
                    }
                }
                Inner::Brotli(ref mut decoder) => {
                    match ready!(Pin::new(decoder).poll_next(cx)) {
                        Some(Ok(chunk)) => {
                            std::task::Poll::Ready(Some(Ok(chunk.freeze())))
                        }
                        Some(Err(e)) => {
                            std::task::Poll::Ready(Some(Err(FetchError::from(e))))
                        }
                        None => std::task::Poll::Ready(None),
                    }
                }
                Inner::Zstd(ref mut decoder) => {
                    match ready!(Pin::new(decoder).poll_next(cx)) {
                        Some(Ok(chunk)) => {
                            std::task::Poll::Ready(Some(Ok(chunk.freeze())))
                        }
                        Some(Err(e)) => {
                            std::task::Poll::Ready(Some(Err(FetchError::from(e))))
                        }
                        None => std::task::Poll::Ready(None),
                    }
                }
                Inner::Deflate(ref mut decoder) => {
                    match ready!(Pin::new(decoder).poll_next(cx)) {
                        Some(Ok(chunk)) => {
                            std::task::Poll::Ready(Some(Ok(chunk.freeze())))
                        }
                        Some(Err(e)) => {
                            std::task::Poll::Ready(Some(Err(FetchError::from(e))))
                        }
                        None => std::task::Poll::Ready(None),
                    }
                }
                Inner::Plain(ref mut stream) => Pin::new(stream).poll_next(cx),
            };
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            Inner::Plain(stream) => stream.size_hint(),
            _ => Default::default(),
        }
    }
}

impl Body for Decoder {
    type Data = Bytes;
    type Error = FetchError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>>
    {
        let this = Pin::into_inner(self);

        loop {
            break match this.inner {
                Inner::Gzip(ref mut decoder) => {
                    match ready!(Pin::new(decoder).poll_next(cx)) {
                        Some(Ok(chunk)) => std::task::Poll::Ready(Some(Ok(
                            hyper::body::Frame::data(chunk.freeze()),
                        ))),
                        Some(Err(e)) => {
                            std::task::Poll::Ready(Some(Err(FetchError::from(e))))
                        }
                        None => std::task::Poll::Ready(None),
                    }
                }
                Inner::Brotli(ref mut decoder) => {
                    match ready!(Pin::new(decoder).poll_next(cx)) {
                        Some(Ok(chunk)) => std::task::Poll::Ready(Some(Ok(
                            hyper::body::Frame::data(chunk.freeze()),
                        ))),
                        Some(Err(e)) => {
                            std::task::Poll::Ready(Some(Err(FetchError::from(e))))
                        }
                        None => std::task::Poll::Ready(None),
                    }
                }
                Inner::Zstd(ref mut decoder) => {
                    match ready!(Pin::new(decoder).poll_next(cx)) {
                        Some(Ok(chunk)) => std::task::Poll::Ready(Some(Ok(
                            hyper::body::Frame::data(chunk.freeze()),
                        ))),
                        Some(Err(e)) => {
                            std::task::Poll::Ready(Some(Err(FetchError::from(e))))
                        }
                        None => std::task::Poll::Ready(None),
                    }
                }
                Inner::Deflate(ref mut decoder) => {
                    match ready!(Pin::new(decoder).poll_next(cx)) {
                        Some(Ok(chunk)) => std::task::Poll::Ready(Some(Ok(
                            hyper::body::Frame::data(chunk.freeze()),
                        ))),
                        Some(Err(e)) => {
                            std::task::Poll::Ready(Some(Err(FetchError::from(e))))
                        }
                        None => std::task::Poll::Ready(None),
                    }
                }
                Inner::Plain(ref mut stream) => {
                    match ready!(Pin::new(stream).poll_next(cx)) {
                        Some(Ok(chunk)) => std::task::Poll::Ready(Some(Ok(
                            hyper::body::Frame::data(chunk),
                        ))),
                        Some(Err(e)) => {
                            std::task::Poll::Ready(Some(Err(FetchError::from(e))))
                        }
                        None => std::task::Poll::Ready(None),
                    }
                }
            };
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.inner {
            Inner::Plain(stream) => stream.0.size_hint(),
            _ => SizeHint::default(),
        }
    }
}
