use crate::http::body::InternalBodyStream;
use crate::http::fetch::errors::FetchError;
use crate::http::headers::HeadersMap;
use async_compression::tokio::bufread::BrotliEncoder;
use async_compression::tokio::bufread::GzipEncoder;
use async_compression::tokio::bufread::ZlibEncoder;
use async_compression::tokio::bufread::ZstdEncoder;
use bytes::Bytes;
use futures::ready;
use futures::Stream;
use hyper::body::Body;
use hyper::body::SizeHint;
use hyper::header::ACCEPT_ENCODING;
use std::pin::Pin;
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_util::io::StreamReader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderType {
    Gzip,
    Brotli,
    Zstd,
    Deflate,
}

impl TryFrom<&str> for EncoderType {
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

#[derive(Debug)]
pub struct StreamEncoder {
    inner: Inner,
    encoder_type: Option<EncoderType>,
}

impl StreamEncoder {
    pub fn encoding(&self) -> Option<EncoderType> {
        self.encoder_type
    }

    pub fn gzip(stream: InternalBodyStream) -> Self {
        Self {
            inner: Inner::Gzip(Box::pin(FramedRead::new(
                GzipEncoder::new(StreamReader::new(stream)),
                BytesCodec::new(),
            ))),
            encoder_type: Some(EncoderType::Gzip),
        }
    }

    pub fn brotli(stream: InternalBodyStream) -> Self {
        Self {
            inner: Inner::Brotli(Box::pin(FramedRead::new(
                BrotliEncoder::new(StreamReader::new(stream)),
                BytesCodec::new(),
            ))),
            encoder_type: Some(EncoderType::Brotli),
        }
    }

    pub fn zstd(stream: InternalBodyStream) -> Self {
        Self {
            inner: Inner::Zstd(Box::pin(FramedRead::new(
                ZstdEncoder::new(StreamReader::new(stream)),
                BytesCodec::new(),
            ))),
            encoder_type: Some(EncoderType::Zstd),
        }
    }

    pub fn deflate(stream: InternalBodyStream) -> Self {
        Self {
            inner: Inner::Deflate(Box::pin(FramedRead::new(
                ZlibEncoder::new(StreamReader::new(stream)),
                BytesCodec::new(),
            ))),
            encoder_type: Some(EncoderType::Deflate),
        }
    }

    pub fn plain(stream: InternalBodyStream) -> Self {
        Self {
            inner: Inner::Plain(stream),
            encoder_type: None,
        }
    }

    /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept-Encoding
    /// Weighted Accept-Encoding values
    /// The following header shows Accept-Encoding preferences using a quality value between 0 (lowest priority) and 1 (highest-priority).
    /// Brotli compression is weighted at 1.0, making br the client's first choice, followed by gzip at 0.8 priority, and then any other content encoding at 0.1:
    /// HTTP
    /// Accept-Encoding: br;q=1.0, gzip;q=0.8, *;q=0.1ƒ
    fn detect_encoding(headers: &HeadersMap) -> Option<EncoderType> {
        let accept_encodings = headers.get_all(ACCEPT_ENCODING.as_str());

        if accept_encodings.is_empty() {
            return None;
        }

        let mut best_encoding = None;
        let mut best_q = -1.0;

        // TODO: Refactor this to use a better parser
        // do benchmarking to see if this is faster
        for accept_encoding in accept_encodings.iter() {
            for encoding_str in accept_encoding.split(',') {
                let encoding_str = encoding_str.trim();
                let mut parts = encoding_str.split(";q=");
                let encoding = parts.next().unwrap_or("");
                let q = parts
                    .next()
                    .and_then(|q_str| q_str.parse::<f32>().ok())
                    .unwrap_or(1.0);

                if q > best_q {
                    if let Ok(encoder_type) = EncoderType::try_from(encoding) {
                        best_encoding = Some(encoder_type);
                        best_q = q;
                    } else if encoding == "*" {
                        best_encoding = Some(EncoderType::Gzip);
                        best_q = q;
                    }
                }
            }
        }

        best_encoding
    }

    // fn detect_encoding(headers: &HeadersMap) -> Option<EncoderType> {
    //     let accept_encodings = headers.get_all(ACCEPT_ENCODING.as_str());

    //     if accept_encodings.is_empty() {
    //         return None;
    //     }

    //     let mut encodings = Vec::new();

    //     for accept_encoding in accept_encodings.iter() {
    //         for encoding_str in accept_encoding.split(',') {
    //             let parts: Vec<&str> = encoding_str.trim().split(";q=").collect();
    //             let encoding = parts[0].trim();
    //             let q = if parts.len() > 1 {
    //                 parts[1].parse::<f32>().unwrap_or(1.0)
    //             } else {
    //                 1.0
    //             };
    //             encodings.push((encoding, q));
    //         }
    //     }

    //     encodings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

    //     for (encoding, _) in encodings {
    //         println!("Encoding: {}", encoding);
    //         if let Ok(encoder_type) = EncoderType::try_from(encoding) {
    //             return Some(encoder_type);
    //         } else if encoding == "*" {
    //             return Some(EncoderType::Gzip);
    //         }
    //     }

    //     None
    // }

    pub fn detect(stream: InternalBodyStream, headers: &HeadersMap) -> Self {
        match Self::detect_encoding(headers) {
            Some(EncoderType::Gzip) => Self::gzip(stream),
            Some(EncoderType::Brotli) => Self::brotli(stream),
            Some(EncoderType::Zstd) => Self::zstd(stream),
            Some(EncoderType::Deflate) => Self::deflate(stream),
            None => Self::plain(stream),
        }
    }
}

type IncomingStreamReader = StreamReader<InternalBodyStream, Bytes>;

#[derive(Debug)]
enum Inner {
    Gzip(Pin<Box<FramedRead<GzipEncoder<IncomingStreamReader>, BytesCodec>>>),
    Brotli(Pin<Box<FramedRead<BrotliEncoder<IncomingStreamReader>, BytesCodec>>>),
    Zstd(Pin<Box<FramedRead<ZstdEncoder<IncomingStreamReader>, BytesCodec>>>),
    Deflate(Pin<Box<FramedRead<ZlibEncoder<IncomingStreamReader>, BytesCodec>>>),
    Plain(InternalBodyStream),
}

impl Stream for StreamEncoder {
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

impl Body for StreamEncoder {
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
            // Inner::Plain(stream) => stream.0.size_hint(),
            _ => SizeHint::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use hyper::header::ACCEPT_ENCODING;
    use kedo_std::BoundedBufferChannel;

    #[tokio::test]
    async fn test_stream_encoder_plain() {
        let mut stream = BoundedBufferChannel::<Vec<u8>>::new(5);
        for i in b"hello".to_vec() {
            stream.try_write(vec![i]).unwrap();
        }
        let encoder = StreamEncoder::plain(InternalBodyStream::new(
            stream.aquire_reader().take().unwrap(),
        ));
        stream.close();

        let mut chunks = Vec::new();
        let mut stream = encoder;
        while let Some(chunk) = stream.next().await {
            chunks.push(chunk);
        }

        assert_eq!(chunks.len(), 5);
        let text = chunks
            .iter()
            .map(|c| c.as_ref().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            text,
            vec![
                &Bytes::from("h"),
                &Bytes::from("e"),
                &Bytes::from("l"),
                &Bytes::from("l"),
                &Bytes::from("o")
            ]
        );
    }

    #[tokio::test]
    async fn test_stream_encoder_detect() {
        let accept = ACCEPT_ENCODING.to_string();
        let headers = HeadersMap::new(vec![(accept.clone(), "gzip".to_string())]);

        let mut stream = BoundedBufferChannel::<Vec<u8>>::new(5);
        let b_stream = InternalBodyStream::new(stream.aquire_reader().unwrap());
        let encoder = StreamEncoder::detect(b_stream, &headers);
        assert!(matches!(encoder.inner, Inner::Gzip(_)));

        let headers = HeadersMap::new(vec![(accept.clone(), "br, deflate".to_string())]);
        let mut stream = BoundedBufferChannel::<Vec<u8>>::new(5);
        let b_stream = InternalBodyStream::new(stream.aquire_reader().unwrap());
        let encoder = StreamEncoder::detect(b_stream, &headers);
        assert!(matches!(encoder.inner, Inner::Brotli(_)));

        let headers = HeadersMap::new(vec![
            (accept.clone(), "gzip;q=0.8".to_string()),
            (accept.clone(), "zstd;q=1.0, deflate".to_string()),
        ]);
        let mut stream = BoundedBufferChannel::<Vec<u8>>::new(5);
        let b_stream = InternalBodyStream::new(stream.aquire_reader().unwrap());
        let encoder = StreamEncoder::detect(b_stream, &headers);
        assert!(matches!(encoder.inner, Inner::Zstd(_)));
    }

    #[tokio::test]
    async fn test_stream_encoder_detect_multiple_encodings() {
        let mut stream = BoundedBufferChannel::<Vec<u8>>::new(5);
        let headers = HeadersMap::new(vec![(
            ACCEPT_ENCODING.to_string(),
            "gzip;q=0.8, br;q=1.0, *;q=0.1".to_string(),
        )]);

        let b_stream = InternalBodyStream::new(stream.aquire_reader().unwrap());
        let encoder = StreamEncoder::detect(b_stream, &headers);
        assert!(encoder.encoding().is_some());
        assert_eq!(encoder.encoding().unwrap(), EncoderType::Brotli);
        assert!(matches!(encoder.inner, Inner::Brotli(_)));
    }
}
