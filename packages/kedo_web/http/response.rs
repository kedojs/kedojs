use super::{
    body::InternalBodyStream,
    decoder::{
        decoder::StreamDecoder,
        encoder::StreamEncoder,
        resource::{DecodedStreamResource, EncodedStreamResource},
    },
    headers::HeadersMap,
};
use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Either, Empty, Full};
use hyper::Uri;
use kedo_core::downcast_state;
use kedo_std::BoundedBufferChannel;
use kedo_utils::downcast_ref;
use rust_jsc::{JSArray, JSContext, JSError, JSObject, JSResult, JSTypedArray, JSValue};
use std::{convert::Infallible, str::FromStr};

impl TryFrom<(&HeadersMap, &JSObject)> for ResponseBody {
    type Error = JSError;

    fn try_from((headers, value): (&HeadersMap, &JSObject)) -> Result<Self, Self::Error> {
        if value.has_property("source") {
            let source = value.get_property("source")?.as_object()?;
            let buffer = JSTypedArray::from(source);
            Ok(ResponseBody::Bytes(Bytes::from(buffer.as_vec()?)))
        } else if value.has_property("stream") {
            let stream = downcast_ref::<BoundedBufferChannel<Vec<u8>>>(
                &value.get_property("stream")?.as_object()?,
            );
            let mut stream = match stream {
                Some(stream) => stream,
                None => return Ok(ResponseBody::None),
            };
            let stream_reader = match stream.as_mut().aquire_reader() {
                Some(reader) => reader,
                None => return Ok(ResponseBody::None),
            };

            let response_body = ResponseBody::EncodedStream(StreamEncoder::detect(
                InternalBodyStream::new(stream_reader),
                headers,
            ));
            return Ok(response_body);
        } else {
            Ok(ResponseBody::None)
        }
    }
}

type HttpBodyResponse = Either<StreamEncoder, BoxBody<bytes::Bytes, Infallible>>;

impl TryInto<hyper::Response<HttpBodyResponse>> for FetchResponse {
    type Error = String;

    fn try_into(self) -> Result<hyper::Response<HttpBodyResponse>, Self::Error> {
        let mut response = hyper::Response::builder().status(self.status);

        for (key, value) in self.headers.into_iter() {
            response = response.header(key, value);
        }

        let body = match self.body {
            ResponseBody::EncodedStream(stream) => {
                let stream = stream.into();
                Either::Left(stream)
            }
            ResponseBody::None => Either::Right(Empty::new().boxed()),
            ResponseBody::Bytes(bytes) => Either::Right(Full::new(bytes).boxed()),
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
    pub(crate) body: ResponseBody,
}

impl FetchResponse {
    pub fn to_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        let state = downcast_state(ctx);

        let value = JSObject::new(ctx);
        let urls = JSArray::new_array(
            ctx,
            &self
                .urls
                .iter()
                .map(|url| JSValue::string(ctx, url.to_string()))
                .collect::<Vec<JSValue>>(),
        )?;

        let status = JSValue::number(ctx, self.status as f64);
        let status_message = JSValue::string(ctx, self.status_message);

        value.set_property("urls", &urls, Default::default())?;
        value.set_property("status", &status, Default::default())?;
        value.set_property("status_message", &status_message, Default::default())?;

        let headers = self.headers.to_value(ctx)?;
        value.set_property("headers", &headers, Default::default())?;

        match self.body {
            ResponseBody::Bytes(_) => todo!(),
            ResponseBody::DecodedStream(stream_decoder) => {
                let decoded_stream_class = state
                    .classes()
                    .get(DecodedStreamResource::CLASS_NAME)
                    .expect("DecodedBodyStream class not found");

                let body_object = decoded_stream_class
                    .object::<StreamDecoder>(ctx, Some(Box::new(stream_decoder)));
                value.set_property("body", &body_object, Default::default())?;
            }
            ResponseBody::EncodedStream(stream_encoder) => {
                let encoded_stream_class = state
                    .classes()
                    .get(EncodedStreamResource::CLASS_NAME)
                    .expect("EncodedBodyStream class not found");

                let body_object = encoded_stream_class
                    .object::<StreamEncoder>(ctx, Some(Box::new(stream_encoder)));
                value.set_property("body", &body_object, Default::default())?;
            }
            _ => {}
        };

        Ok(value.into())
    }

    pub fn from_object(ctx: &JSContext, value: &JSObject) -> JSResult<FetchResponse> {
        let url: String = value.get_property("url")?.as_string()?.to_string();
        let url = match Uri::from_str(url.as_str()) {
            Ok(url) => url,
            Err(_) => return Err(JSError::new_typ(&ctx, "Invalid URL")?),
        };

        let status = value.get_property("status")?.as_number()? as u16;
        let status_message = value
            .get_property("status_message")?
            .as_string()?
            .to_string();

        let headers = JSArray::new(value.get_property("headers")?.as_object()?);
        let headers_map = HeadersMap::try_from(headers)?;
        let body = ResponseBody::try_from((&headers_map, value))?;

        Ok(FetchResponse {
            urls: vec![url],
            status,
            status_message,
            headers: headers_map,
            aborted: false,
            body,
        })
    }
}
