use super::headers::HeadersMapExt;
use crate::DecodedStreamResource;
use bytes::Bytes;
use kedo_core::{define_exports, downcast_state};
use kedo_macros::js_class;
use kedo_std::{
    BoundedBufferChannel, FetchError, FetchRequest, FetchRequestBuilder, HeadersMap,
    IncomingBodyStream, RequestBody, RequestEvent, RequestRedirect, StreamDecoder,
};
use kedo_utils::downcast_ref;
use rust_jsc::{
    callback, JSArray, JSContext, JSError, JSObject, JSResult, JSTypedArray, JSValue,
};
use std::mem::ManuallyDrop;

pub trait FetchRequestExt {
    fn from_value(value: &JSValue, ctx: &JSContext) -> JSResult<FetchRequest>;
    fn from_event(value: RequestEvent) -> Result<FetchRequest, FetchError>;
}

impl FetchRequestExt for FetchRequest {
    fn from_value(value: &JSValue, ctx: &JSContext) -> JSResult<FetchRequest> {
        fetch_request_from_value(value, ctx)
    }

    fn from_event(value: RequestEvent) -> Result<FetchRequest, FetchError> {
        let mut headers = HeadersMap::default();
        for (name, value) in value.req.headers() {
            let value_str = value.to_str();
            if let Ok(value_str) = value_str {
                headers.append(name.as_str(), value_str);
            }
        }

        // check keep alive from headers
        let keep_alive = headers
            .get("connection")
            .map(|value| value.to_lowercase() == "keep-alive")
            .unwrap_or(false);

        let method = value.req.method().as_str().to_string();
        let uri = value.req.uri().clone();
        let body_stream = IncomingBodyStream::new(value.req.into_body());
        let decoded_body = StreamDecoder::detect(body_stream, &headers);

        let request = FetchRequestBuilder::new()
            .method(method)
            .uri(uri)
            .headers(headers)
            .keep_alive(keep_alive)
            .body(RequestBody::Stream(Some(decoded_body)))
            .build()
            .map_err(|e| FetchError {
                message: "Failed to build fetch request".into(),
                inner: Some(e.into()),
            })?;
        Ok(request)
    }
}

pub trait RequestBodyExt {
    fn to_object(self, ctx: &JSContext) -> JSResult<Option<JSObject>>;
    fn from_object(obj: &JSObject) -> Result<RequestBody, JSError>;
}

impl RequestBodyExt for RequestBody {
    fn to_object(self, ctx: &JSContext) -> JSResult<Option<JSObject>> {
        match self {
            RequestBody::None => Ok(None),
            RequestBody::Bytes(bytes) => {
                let mut bytes: ManuallyDrop<Vec<u8>> = ManuallyDrop::new(bytes.to_vec());
                let buffer = JSTypedArray::with_bytes(
                    ctx,
                    bytes.as_mut_slice(),
                    rust_jsc::JSTypedArrayType::Uint8Array,
                )?;
                Ok(Some(buffer.into()))
            }
            RequestBody::Stream(Some(stream)) => {
                let state = downcast_state(ctx);
                let class = state
                    .classes()
                    .get(DecodedStreamResource::CLASS_NAME)
                    .unwrap();

                let body_object =
                    class.object::<StreamDecoder>(ctx, Some(Box::new(stream)));
                Ok(Some(body_object))
            }
            RequestBody::Stream(None) => Ok(None),
        }
    }

    fn from_object(obj: &JSObject) -> Result<RequestBody, JSError> {
        if obj.has_property("source") {
            let source = obj.get_property("source")?.as_object()?;
            let buffer = JSTypedArray::from(source);
            Ok(RequestBody::Bytes(Bytes::from(buffer.as_vec()?)))
        } else if obj.has_property("stream") {
            let stream = downcast_ref::<BoundedBufferChannel<Vec<u8>>>(
                &obj.get_property("stream")?.as_object()?,
            );
            Ok(stream
                .map(|mut s| {
                    RequestBody::Stream(
                        s.as_mut()
                            .aquire_reader()
                            .map(StreamDecoder::internal_stream),
                    )
                })
                .unwrap_or(RequestBody::None))
        } else {
            Ok(RequestBody::None)
        }
    }
}

fn fetch_request_from_value(value: &JSValue, ctx: &JSContext) -> JSResult<FetchRequest> {
    if !value.is_object() {
        return Err(JSError::new_typ(&ctx, "Missing highWaterMark argument")?);
    }

    let request = value.as_object()?;
    let method = request.get_property("method")?.as_string()?.to_string();
    let url = request.get_property("url")?.as_string()?.to_string();
    let redirect = request
        .get_property("redirect")
        .and_then(|v| Ok(RequestRedirect::from(v.as_number()? as u8)))
        .unwrap_or(RequestRedirect::Follow);
    let keep_alive = request
        .get_property("keep_alive")
        .and_then(|v| Ok(v.as_boolean()))
        .unwrap_or(false);
    let uri = url
        .parse::<hyper::Uri>()
        .map_err(|_| JSError::new_typ(&ctx, "Invalid URL").unwrap())?;
    let header_list = request.get_property("header_list")?.as_object()?;
    if !header_list.is_array() {
        return Err(JSError::new_typ(&ctx, "header_list must be an array")?);
    }

    let headers = HeadersMap::from_array(JSArray::new(header_list))?;
    let body = RequestBody::from_object(&request)?;
    let request = FetchRequestBuilder::new()
        .method(method)
        .uri(uri)
        .headers(headers)
        .keep_alive(keep_alive)
        .redirect(redirect)
        .body(body)
        .build()
        .map_err(|e| JSError::new_typ(&ctx, e).unwrap())?;

    Ok(request)
}

#[js_class(
    resource = FetchRequest,
)]
pub struct FetchRequestResource {}

#[callback]
fn op_http_request_method(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    request: JSObject,
) -> JSResult<JSValue> {
    let client = downcast_ref::<FetchRequest>(&request);
    let method = match client {
        Some(client) => client.method.clone(),
        None => return Err(JSError::new_typ(&ctx, "[Op:Method] Invalid request")?),
    };

    Ok(JSValue::string(&ctx, method))
}

#[callback]
fn op_http_request_uri(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    request: JSObject,
) -> JSResult<JSValue> {
    let client = downcast_ref::<FetchRequest>(&request);
    let uri = match client {
        Some(client) => client.uri.to_string(),
        None => return Err(JSError::new_typ(&ctx, "[Op:Uri] Invalid request")?),
    };

    // if doesn't have authority, and scheme add them
    if !uri.contains("://") {
        return Ok(JSValue::string(&ctx, format!("http://localhost{}", uri)));
    }

    Ok(JSValue::string(&ctx, uri))
}

#[callback]
fn op_http_request_headers(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    request: JSObject,
) -> JSResult<JSValue> {
    let client = downcast_ref::<FetchRequest>(&request);
    let headers = match client {
        Some(client) => client.headers.into_value(&ctx)?,
        None => return Err(JSError::new_typ(&ctx, "[Op:Headers] Invalid request")?),
    };

    Ok(headers)
}

#[callback]
fn op_http_request_keep_alive(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    request: JSObject,
) -> JSResult<JSValue> {
    let client = downcast_ref::<FetchRequest>(&request);
    let keep_alive = match client {
        Some(client) => client.keep_alive,
        None => return Err(JSError::new_typ(&ctx, "[Op:KeepAlive] Invalid request")?),
    };

    Ok(JSValue::boolean(&ctx, keep_alive))
}

#[callback]
fn op_http_request_redirect(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    request: JSObject,
) -> JSResult<JSValue> {
    let client = downcast_ref::<FetchRequest>(&request);
    let redirect = match client {
        Some(client) => client.redirect as u8,
        None => return Err(JSError::new_typ(&ctx, "[Op:Redirect] Invalid request")?),
    };

    Ok(JSValue::number(&ctx, redirect as f64))
}

#[callback]
fn op_http_request_redirect_count(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    request: JSObject,
) -> JSResult<JSValue> {
    let client = downcast_ref::<FetchRequest>(&request);
    let redirect_count = match client {
        Some(client) => client.redirect_count,
        None => {
            return Err(JSError::new_typ(
                &ctx,
                "[Op:RedirectCount] Invalid request",
            )?)
        }
    };

    Ok(JSValue::number(&ctx, redirect_count as f64))
}

#[callback]
fn op_http_request_body(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    request: JSObject,
) -> JSResult<JSValue> {
    let client = downcast_ref::<FetchRequest>(&request);
    let mut client = match client {
        Some(client) => client,
        None => return Err(JSError::new_typ(&ctx, "[Op:Body] Invalid request")?),
    };

    if let Some(object) = client.body.take().to_object(&ctx)? {
        return Ok(object.into());
    }

    Ok(JSValue::null(&ctx))
}

pub struct FetchRequestOps {}

define_exports!(
    FetchRequestOps,
    @template[],
    @function[
        op_http_request_method,
        op_http_request_uri,
        op_http_request_headers,
        op_http_request_keep_alive,
        op_http_request_redirect,
        op_http_request_redirect_count,
        op_http_request_body,
    ]
);

#[cfg(test)]
mod tests {}
