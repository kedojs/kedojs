use super::headers::HeadersMapExt;
use crate::{stream_codec::EncodedStreamResource, DecodedStreamResource};
use bytes::Bytes;
use hyper::Uri;
use kedo_core::downcast_state;
use kedo_std::{
    BoundedBufferChannel, HttpResponse, InternalBodyStream, ResponseBody, StreamDecoder,
    StreamEncoder,
};
use kedo_utils::downcast_ref;
use rust_jsc::{JSArray, JSContext, JSError, JSObject, JSResult, JSTypedArray, JSValue};
use std::str::FromStr;

pub trait ResponseBodyExt {
    fn from_value(
        headers: &hyper::HeaderMap,
        object: &JSObject,
    ) -> JSResult<ResponseBody>;
}

impl ResponseBodyExt for ResponseBody {
    fn from_value(headers: &hyper::HeaderMap, object: &JSObject) -> JSResult<Self> {
        if object.has_property("source") {
            let source = object.get_property("source")?.as_object()?;
            let buffer = JSTypedArray::from(source);
            Ok(ResponseBody::Bytes(Bytes::from(buffer.as_vec()?)))
        } else if object.has_property("stream") {
            let stream = downcast_ref::<BoundedBufferChannel<Vec<u8>>>(
                &object.get_property("stream")?.as_object()?,
            );
            let mut stream = match stream {
                Some(stream) => stream,
                None => return Ok(ResponseBody::None),
            };
            let reader = match stream.as_mut().aquire_reader() {
                Some(reader) => reader,
                None => return Ok(ResponseBody::None),
            };

            let response_body = ResponseBody::EncodedStream(StreamEncoder::detect(
                InternalBodyStream::new(reader),
                headers,
            ));
            return Ok(response_body);
        } else {
            Ok(ResponseBody::None)
        }
    }
}

pub trait FetchResponseExt<T> {
    fn to_value(self, ctx: &JSContext) -> JSResult<JSValue>;
    fn from_object(ctx: &JSContext, value: &JSObject) -> JSResult<T>;
}

impl FetchResponseExt<Self> for HttpResponse {
    fn to_value(mut self, ctx: &JSContext) -> JSResult<JSValue> {
        let state = downcast_state(ctx);

        let value = JSObject::new(ctx);
        let urls = JSArray::new_array(
            ctx,
            &self
                .urls()
                .iter()
                .map(|url| JSValue::string(ctx, url.to_string()))
                .collect::<Vec<JSValue>>(),
        )?;

        let status = JSValue::number(ctx, self.status().as_u16() as f64);
        let status_message =
            JSValue::string(ctx, self.status().canonical_reason().unwrap_or(""));

        value.set_property("urls", &urls, Default::default())?;
        value.set_property("status", &status, Default::default())?;
        value.set_property("status_message", &status_message, Default::default())?;

        let headers = self.headers().into_value(ctx)?;
        value.set_property("headers", &headers, Default::default())?;

        match self.take_body() {
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

    fn from_object(ctx: &JSContext, value: &JSObject) -> JSResult<HttpResponse> {
        let url: String = value.get_property("url")?.as_string()?.to_string();
        let url = match Uri::from_str(url.as_str()) {
            Ok(url) => url,
            Err(_) => return Err(JSError::new_typ(&ctx, "Invalid URL")?),
        };

        let status = value.get_property("status")?.as_number()? as u16;
        let headers = JSArray::new(value.get_property("headers")?.as_object()?);
        let headers = hyper::header::HeaderMap::from_array(headers)?;
        let body = ResponseBody::from_value(&headers, value)?;

        let builder = match HttpResponse::builder().status_code(status) {
            Ok(builder) => builder,
            Err(e) => return Err(JSError::new_typ(ctx, e.to_string())?),
        };
        let response = match builder.headers(headers).body(body).url(url).build() {
            Ok(response) => response,
            Err(e) => return Err(JSError::new_typ(ctx, e.to_string())?),
        };
        Ok(response)
    }
}
