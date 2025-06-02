use super::headers::HeadersMapExt;
use crate::DecodedStreamResource;
use bytes::Bytes;
use kedo_core::{define_exports, downcast_state};
use kedo_macros::js_class;
use kedo_std::{
    BufferChannel, FetchError, HttpRequest, HttpRequestBuilder, HttpRequestEvent,
    IncomingBodyStream, RequestBody, RequestRedirect, StreamDecoder,
    UnboundedBufferChannel,
};
use kedo_utils::downcast_ref;
use rust_jsc::{
    callback, JSArray, JSContext, JSError, JSObject, JSResult, JSTypedArray, JSValue,
};
use std::mem::ManuallyDrop;

pub trait HttpRequestExt {
    fn from_value(value: &JSValue, ctx: &JSContext) -> JSResult<HttpRequest>;
    fn from_event(value: HttpRequestEvent) -> Result<HttpRequest, FetchError>;
}

impl HttpRequestExt for HttpRequest {
    fn from_value(value: &JSValue, ctx: &JSContext) -> JSResult<HttpRequest> {
        if !value.is_object() {
            return Err(JSError::new_typ(&ctx, "Request must be an object")?);
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
        let uri = match url.parse::<hyper::Uri>() {
            Ok(uri) => uri,
            Err(_) => return Err(JSError::new_typ(&ctx, "Invalid URL")?),
        };
        let header_list = request.get_property("header_list")?.as_object()?;

        let headers = hyper::HeaderMap::from_array(JSArray::new(header_list))?;
        let body = RequestBody::from_object(&request)?;
        let request = match HttpRequestBuilder::new()
            .method(method)
            .uri(uri)
            .headers(headers)
            .keep_alive(keep_alive)
            .redirect(redirect)
            .body(body)
            .build()
        {
            Ok(request) => request,
            Err(_) => return Err(JSError::new_typ(&ctx, "Invalid request")?),
        };

        Ok(request)
    }

    fn from_event(value: HttpRequestEvent) -> Result<HttpRequest, FetchError> {
        // Clone necessary parts before consuming the body
        let method = value.req.method().clone();
        let uri = value.req.uri().clone();
        let headers = value.req.headers().clone();

        // check keep alive from headers
        let keep_alive = headers
            .get("connection")
            .map(|value| value.to_str().unwrap_or("").to_lowercase() == "keep-alive")
            .unwrap_or(false);

        let body_stream = IncomingBodyStream::new(value.req.into_body());
        let decoded_body = StreamDecoder::detect_encoding(body_stream, &headers);

        let request = HttpRequestBuilder::new()
            .method(method.to_string())
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
                    .expect("DecodedStreamResource class not found");

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
            let stream = downcast_ref::<UnboundedBufferChannel<Vec<u8>>>(
                &obj.get_property("stream")?.as_object()?,
            );
            Ok(stream
                .map(|mut s| {
                    RequestBody::Stream(
                        s.as_mut()
                            .acquire_reader()
                            .map(StreamDecoder::internal_stream),
                    )
                })
                .unwrap_or(RequestBody::None))
        } else {
            Ok(RequestBody::None)
        }
    }
}

#[js_class(
    resource = HttpRequest,
)]
pub struct FetchRequestResource {}

#[js_class(
    resource = HttpRequest,
)]
pub struct HttpRequestResource {}

#[callback]
fn op_http_request_method(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    request: JSObject,
) -> JSResult<JSValue> {
    let client = downcast_ref::<HttpRequest>(&request);
    let method = match client {
        Some(client) => client.method().to_string(),
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
    let client = downcast_ref::<HttpRequest>(&request);
    let uri = match client {
        Some(client) => client.uri().to_string(),
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
    let client = downcast_ref::<HttpRequest>(&request);
    let headers = match client {
        Some(client) => client.headers().into_value(&ctx)?,
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
    let client = downcast_ref::<HttpRequest>(&request);
    let keep_alive = match client {
        Some(client) => client.keep_alive(),
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
    let client = downcast_ref::<HttpRequest>(&request);
    let redirect: u8 = match client {
        Some(client) => client.redirect().into(),
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
    let client = downcast_ref::<HttpRequest>(&request);
    let redirect_count = match client {
        Some(client) => client.redirect_count(),
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
    let client = downcast_ref::<HttpRequest>(&request);
    let mut client = match client {
        Some(client) => client,
        None => return Err(JSError::new_typ(&ctx, "[Op:Body] Invalid request")?),
    };

    if let Some(object) = client.take_body().to_object(&ctx)? {
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
