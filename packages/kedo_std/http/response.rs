use super::{
    body::HttpBody, decoder::StreamDecoder, encoder::StreamEncoder, headers::HeadersMap,
};
use crate::FetchError;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use hyper::Uri;

impl TryInto<hyper::Response<HttpBody>> for FetchResponse {
    type Error = String;

    fn try_into(self) -> Result<hyper::Response<HttpBody>, Self::Error> {
        let mut response = hyper::Response::builder().status(self.status);

        for (key, value) in self.headers.into_iter() {
            response = response.header(key, value);
        }

        let body = match self.body {
            ResponseBody::EncodedStream(stream) => stream.boxed(),
            ResponseBody::None => Empty::new()
                .map_err(|_| FetchError::new("Http Empty Body Error"))
                .boxed(),
            ResponseBody::Bytes(bytes) => Full::new(bytes)
                .map_err(|_| FetchError::new("Http Full Body Error"))
                .boxed(),
            _ => return Err("Invalid response body".to_string()),
        };

        Ok(response.body(body).map_err(|e| e.to_string())?)
    }
}

#[derive(Debug)]
pub enum ResponseBody {
    None,
    Bytes(Bytes),
    DecodedStream(StreamDecoder),
    EncodedStream(StreamEncoder),
}

pub struct FetchResponse {
    pub urls: Vec<Uri>,
    pub status: u16,
    pub status_message: String,
    pub headers: HeadersMap,
    pub aborted: bool,
    pub body: ResponseBody,
}
