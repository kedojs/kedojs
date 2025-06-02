use crate::http::body::IncomingBodyStream;
use crate::http::errors::FetchError;
use crate::UnboundedBufferChannelReader;
use async_compression::tokio::bufread::BrotliDecoder;
use async_compression::tokio::bufread::GzipDecoder;
use async_compression::tokio::bufread::ZlibDecoder;
use async_compression::tokio::bufread::ZstdDecoder;
use bytes::Bytes;
use futures::ready;
use futures::Stream;
use hyper::body::Body;
use hyper::body::SizeHint;
use hyper::header::CONTENT_ENCODING;
use hyper::header::CONTENT_LENGTH;
use std::pin::Pin;
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_util::io::StreamReader;

/// Content encoding types supported by StreamDecoder
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

type IncomingStreamReader = StreamReader<IncomingBodyStream, Bytes>;

#[derive(Debug)]
pub struct StreamDecoder {
    inner: Inner,
}

#[derive(Debug)]
enum Inner {
    Gzip(Pin<Box<FramedRead<GzipDecoder<IncomingStreamReader>, BytesCodec>>>),
    Brotli(Pin<Box<FramedRead<BrotliDecoder<IncomingStreamReader>, BytesCodec>>>),
    Zstd(Pin<Box<FramedRead<ZstdDecoder<IncomingStreamReader>, BytesCodec>>>),
    Deflate(Pin<Box<FramedRead<ZlibDecoder<IncomingStreamReader>, BytesCodec>>>),
    Plain(IncomingBodyStream),
    InternalStream(UnboundedBufferChannelReader<Vec<u8>>),
}

impl StreamDecoder {
    pub fn gzip(stream: IncomingBodyStream) -> Self {
        let encoder = GzipDecoder::new(StreamReader::new(stream));
        let inner = FramedRead::new(encoder, BytesCodec::new());
        Self {
            inner: Inner::Gzip(Box::pin(inner)),
        }
    }

    pub fn brotli(stream: IncomingBodyStream) -> Self {
        let encoder = BrotliDecoder::new(StreamReader::new(stream));
        let inner = FramedRead::new(encoder, BytesCodec::new());
        Self {
            inner: Inner::Brotli(Box::pin(inner)),
        }
    }

    pub fn zstd(stream: IncomingBodyStream) -> Self {
        let encoder = ZstdDecoder::new(StreamReader::new(stream));
        let inner = FramedRead::new(encoder, BytesCodec::new());
        Self {
            inner: Inner::Zstd(Box::pin(inner)),
        }
    }

    pub fn deflate(stream: IncomingBodyStream) -> Self {
        let encoder = ZlibDecoder::new(StreamReader::new(stream));
        let inner = FramedRead::new(encoder, BytesCodec::new());
        Self {
            inner: Inner::Deflate(Box::pin(inner)),
        }
    }

    pub fn plain(stream: IncomingBodyStream) -> Self {
        Self {
            inner: Inner::Plain(stream),
        }
    }

    pub fn internal_stream(stream: UnboundedBufferChannelReader<Vec<u8>>) -> Self {
        Self {
            inner: Inner::InternalStream(stream),
        }
    }

    /// Efficiently detects the appropriate decoder based on Content-Encoding headers.
    ///
    /// According to the HTTP standard, Content-Encoding refers to the encoded form of the data.
    /// This implementation handles the first encoding value found. Multiple encodings
    /// (e.g. "deflate, gzip") are not yet supported.
    ///
    /// # Arguments
    /// * `stream` - The incoming body stream
    /// * `headers` - The HTTP headers from the request/response
    ///
    /// # Returns
    /// A StreamDecoder configured with the appropriate decoder
    pub fn detect_encoding(
        stream: IncomingBodyStream,
        headers: &hyper::header::HeaderMap,
    ) -> Self {
        // For empty bodies, use plain decoder regardless of encoding headers
        if headers
            .get(CONTENT_LENGTH)
            .map(|v| v == "0")
            .unwrap_or(false)
        {
            return Self::plain(stream);
        }

        // TODO: Handle Transfer-Encoding header if needed
        // Find the Content-Encoding header if it exists
        if let Some(encoding) = headers.get(CONTENT_ENCODING) {
            // Only process the first encoding value for now
            // TODO: Support multiple encodings (e.g., "deflate, gzip") in the future
            if let Ok(encoding_str) = encoding.to_str() {
                let first_encoding = encoding_str
                    .split(',')
                    .next()
                    .map(str::trim)
                    .unwrap_or_default();

                // Match on the first encoding type
                return match first_encoding {
                    "gzip" => Self::gzip(stream),
                    "br" => Self::brotli(stream),
                    "zstd" => Self::zstd(stream),
                    "deflate" => Self::deflate(stream),
                    // If encoding is empty or not supported, use plain
                    _ => Self::plain(stream),
                };
            }
        }

        Self::plain(stream)
    }

    // Helper method to poll the inner stream and handle common result mapping
    #[inline]
    fn poll_compressed<S>(
        stream: Pin<&mut S>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<Bytes, FetchError>>>
    where
        S: Stream<Item = Result<bytes::BytesMut, std::io::Error>> + ?Sized,
    {
        match ready!(stream.poll_next(cx)) {
            Some(Ok(chunk)) => std::task::Poll::Ready(Some(Ok(chunk.freeze()))),
            Some(Err(e)) => std::task::Poll::Ready(Some(Err(FetchError::from(e)))),
            None => std::task::Poll::Ready(None),
        }
    }
}

impl Stream for StreamDecoder {
    type Item = Result<Bytes, FetchError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = std::pin::Pin::into_inner(self);

        loop {
            break match this.inner {
                Inner::Gzip(ref mut decoder) => {
                    Self::poll_compressed(Pin::new(decoder), cx)
                }
                Inner::Brotli(ref mut decoder) => {
                    Self::poll_compressed(Pin::new(decoder), cx)
                }
                Inner::Zstd(ref mut decoder) => {
                    Self::poll_compressed(Pin::new(decoder), cx)
                }
                Inner::Deflate(ref mut decoder) => {
                    Self::poll_compressed(Pin::new(decoder), cx)
                }
                Inner::Plain(ref mut stream) => Pin::new(stream).poll_next(cx),
                Inner::InternalStream(ref mut stream) => {
                    match Pin::new(stream).poll_next(cx) {
                        std::task::Poll::Ready(Some(chunk)) => {
                            std::task::Poll::Ready(Some(Ok(Bytes::from(chunk))))
                        }
                        std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
                        std::task::Poll::Pending => std::task::Poll::Pending,
                    }
                }
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

impl Body for StreamDecoder {
    type Data = Bytes;
    type Error = FetchError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>>
    {
        let this = self.get_mut();

        // Reuse the poll_next implementation and wrap the result in a Frame
        match Pin::new(this).poll_next(cx) {
            std::task::Poll::Ready(Some(Ok(data))) => {
                std::task::Poll::Ready(Some(Ok(hyper::body::Frame::data(data))))
            }
            std::task::Poll::Ready(Some(Err(e))) => std::task::Poll::Ready(Some(Err(e))),
            std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.inner {
            Inner::Plain(stream) => {
                let (lower, upper) = stream.size_hint();
                let mut hint = SizeHint::new();
                if lower > 0 {
                    hint.set_lower(lower as u64);
                }
                if let Some(upper) = upper {
                    hint.set_upper(upper as u64);
                }
                hint
            }
            _ => SizeHint::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::body::IncomingBodyStream;

    // Helper to create an empty IncomingBodyStream for testing
    fn create_empty_stream() -> IncomingBodyStream {
        IncomingBodyStream::new_test()
    }

    // Helper to create headers with the specified encoding
    fn create_mock_headers(encoding: Option<&str>) -> hyper::HeaderMap {
        let mut headers = hyper::HeaderMap::new();
        if let Some(enc) = encoding {
            headers.insert(CONTENT_ENCODING, enc.parse().unwrap());
        }
        headers
    }

    #[tokio::test]
    async fn test_detect_from_headers() {
        // Test with gzip encoding
        let headers = create_mock_headers(Some("gzip"));
        let decoder = StreamDecoder::detect_encoding(create_empty_stream(), &headers);
        assert!(matches!(decoder.inner, Inner::Gzip(_)));

        // Test with no encoding
        let headers = create_mock_headers(None);
        let decoder = StreamDecoder::detect_encoding(create_empty_stream(), &headers);
        assert!(matches!(decoder.inner, Inner::Plain(_)));

        // Test with content-length: 0 and encoding
        let mut headers = create_mock_headers(Some("gzip"));
        headers.insert(CONTENT_LENGTH, "0".parse().unwrap());
        let decoder = StreamDecoder::detect_encoding(create_empty_stream(), &headers);
        assert!(matches!(decoder.inner, Inner::Plain(_)));
    }

    #[tokio::test]
    async fn test_detect_encoding() {
        let mut headers = hyper::header::HeaderMap::new();

        // Test with gzip encoding
        headers.insert(CONTENT_ENCODING, "gzip".parse().unwrap());
        let decoder = StreamDecoder::detect_encoding(create_empty_stream(), &headers);
        assert!(matches!(decoder.inner, Inner::Gzip(_)));

        // Test with brotli encoding
        headers.clear();
        headers.insert(CONTENT_ENCODING, "br".parse().unwrap());
        let decoder = StreamDecoder::detect_encoding(create_empty_stream(), &headers);
        assert!(matches!(decoder.inner, Inner::Brotli(_)));

        // Test with multiple encodings - should use the first one
        headers.clear();
        headers.insert(CONTENT_ENCODING, "deflate, gzip".parse().unwrap());
        let decoder = StreamDecoder::detect_encoding(create_empty_stream(), &headers);
        assert!(matches!(decoder.inner, Inner::Deflate(_)));

        // Test with empty content-length
        headers.clear();
        headers.insert(CONTENT_ENCODING, "gzip".parse().unwrap());
        headers.insert(CONTENT_LENGTH, "0".parse().unwrap());
        let decoder = StreamDecoder::detect_encoding(create_empty_stream(), &headers);
        assert!(matches!(decoder.inner, Inner::Plain(_)));

        // Test with no encoding
        headers.clear();
        let decoder = StreamDecoder::detect_encoding(create_empty_stream(), &headers);
        assert!(matches!(decoder.inner, Inner::Plain(_)));

        // Test with unsupported encoding
        headers.clear();
        headers.insert(CONTENT_ENCODING, "dcb".parse().unwrap());
        let decoder = StreamDecoder::detect_encoding(create_empty_stream(), &headers);
        assert!(matches!(decoder.inner, Inner::Plain(_)));
    }

    #[tokio::test]
    async fn test_decoder_type_from_str() {
        assert!(matches!(
            DecoderType::try_from("gzip").unwrap(),
            DecoderType::Gzip
        ));
        assert!(matches!(
            DecoderType::try_from("br").unwrap(),
            DecoderType::Brotli
        ));
        assert!(matches!(
            DecoderType::try_from("zstd").unwrap(),
            DecoderType::Zstd
        ));
        assert!(matches!(
            DecoderType::try_from("deflate").unwrap(),
            DecoderType::Deflate
        ));
        assert!(DecoderType::try_from("").is_err());
        assert!(DecoderType::try_from("unknown").is_err());
    }
}
