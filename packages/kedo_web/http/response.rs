use super::headers::HeadersMapExt;
use crate::{stream_codec::EncodedStreamResource, DecodedStreamResource};
use bytes::Bytes;
use hyper::Uri;
use kedo_core::downcast_state;
use kedo_std::{
    BoundedBufferChannel, FetchResponse, HeadersMap, InternalBodyStream, ResponseBody,
    StreamDecoder, StreamEncoder,
};
use kedo_utils::downcast_ref;
use rust_jsc::{JSArray, JSContext, JSError, JSObject, JSResult, JSTypedArray, JSValue};
use std::str::FromStr;

pub trait ResponseBodyExt {
    fn from_value(headers: &HeadersMap, object: &JSObject) -> JSResult<ResponseBody>;
}

impl ResponseBodyExt for ResponseBody {
    fn from_value(headers: &HeadersMap, object: &JSObject) -> JSResult<Self> {
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

pub trait FetchResponseExt {
    fn to_value(self, ctx: &JSContext) -> JSResult<JSValue>;
    fn from_object(ctx: &JSContext, value: &JSObject) -> JSResult<FetchResponse>;
    fn from_args(ctx: &JSContext, args: &[JSValue]) -> JSResult<FetchResponse>;
}

impl FetchResponseExt for FetchResponse {
    fn to_value(self, ctx: &JSContext) -> JSResult<JSValue> {
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

        let headers = self.headers.into_value(ctx)?;
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

    fn from_object(ctx: &JSContext, value: &JSObject) -> JSResult<FetchResponse> {
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
        let headers_map = HeadersMap::from_array(headers)?;
        let body = ResponseBody::from_value(&headers_map, value)?;

        Ok(FetchResponse {
            urls: vec![url],
            status,
            status_message,
            headers: headers_map,
            aborted: false,
            body,
        })
    }

    /// Create a new FetchResponse from JS arguments
    /// Arguments:
    /// 0: url: string
    /// 1: status: number
    /// 2: status_message: string
    /// 3: headers: Headers
    /// 4: body: Body
    /// Returns: FetchResponse
    fn from_args(ctx: &JSContext, args: &[JSValue]) -> JSResult<FetchResponse> {
        let url: String = args[0].as_string()?.to_string();
        let url = match Uri::from_str(url.as_str()) {
            Ok(url) => url,
            Err(_) => return Err(JSError::new_typ(&ctx, "Invalid URL")?),
        };

        let status = args[1].as_number()? as u16;

        let headers = JSArray::new(args[3].as_object()?);
        let headers_map = HeadersMap::from_array(headers)?;
        let body = match args.get(4) {
            Some(body) => ResponseBody::from_value(&headers_map, &body.as_object()?)?,
            None => ResponseBody::None,
        };

        Ok(FetchResponse {
            urls: vec![url],
            status,
            status_message: "".to_string(),
            headers: headers_map,
            aborted: false,
            body,
        })
    }
}
