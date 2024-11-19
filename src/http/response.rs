use std::mem::ManuallyDrop;

use crate::{
    class_table::ClassTable,
    context::downcast_state,
    http::fetch_errors::FetchError,
    job::{AsyncJobQueue, NativeJob},
    utils::{downcast_ref, drop_ptr},
};

use bytes::Bytes;
use futures::StreamExt;
use hyper::Uri;
use rust_jsc::{
    callback, class::ClassError, constructor, finalize, JSArray, JSClass,
    JSClassAttribute, JSContext, JSError, JSObject, JSPromise, JSResult, JSTypedArray,
    JSValue, PrivateData,
};

use super::{fetch_decoder::Decoder, HeadersMap};

pub struct FetchResponse {
    // pub url: Uri,
    pub urls: Vec<Uri>,
    pub status: u16,
    pub status_message: String,
    pub headers: HeadersMap,
    pub aborted: bool,
    pub(crate) body: Option<Box<Decoder>>,
}

impl FetchResponse {
    pub fn to_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        let state = downcast_state::<AsyncJobQueue>(ctx);
        let class = state
            .classes()
            .get(JSResponseBodyStreamResource::CLASS_NAME)
            .unwrap();

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

        let mut response_headers: Vec<JSValue> = vec![];
        for (key, value) in self.headers.into_iter() {
            let key = JSValue::string(ctx, key);
            let value = JSValue::string(ctx, value);
            let header = JSArray::new_array(ctx, &[key, value])?;
            response_headers.push(header.into());
        }
        let headers = JSArray::new_array(ctx, response_headers.as_slice())?;
        value.set_property("headers", &headers, Default::default())?;

        match self.body {
            Some(stream) => {
                let body_object = class.object::<Decoder>(ctx, Some(stream));
                value.set_property("body", &body_object, Default::default())?;
            }
            None => {}
        };

        Ok(value.into())
    }
}

pub struct JSResponseBodyStreamResource {}

impl JSResponseBodyStreamResource {
    pub const CLASS_NAME: &'static str = "ResponseBodyStream";
    // pub const PROTO_NAME: &'static str = "ResponseBodyStreamPrototype";

    pub fn init_class(manaager: &mut ClassTable) -> Result<(), ClassError> {
        let builder = JSClass::builder(Self::CLASS_NAME);
        let class = builder
            .call_as_constructor(Some(Self::constructor))
            .set_finalize(Some(Self::finalize))
            .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
            .build()?;

        manaager.insert(class);
        Ok(())
    }

    /// finalize is called when the object is being garbage collected.
    /// This is the place to clean up any resources that the object may hold.
    #[finalize]
    fn finalize(data_ptr: PrivateData) {
        drop_ptr::<Decoder>(data_ptr);
    }

    #[constructor]
    fn constructor(
        ctx: JSContext,
        constructor: JSObject,
        _args: &[JSValue],
    ) -> JSResult<JSValue> {
        let state = downcast_state::<AsyncJobQueue>(&ctx);
        let class = state
            .classes()
            .get(JSResponseBodyStreamResource::CLASS_NAME)
            .unwrap();

        let object = class.object::<Decoder>(&ctx, None);
        object.set_prototype(&constructor);
        Ok(object.into())
    }
}

#[callback]
pub fn op_read_response_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let resource_args = args
        .get(0)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing arguments").unwrap())?
        .as_object()?;
    let state = downcast_state::<AsyncJobQueue>(&ctx);
    let resource = downcast_ref::<Decoder>(&resource_args);
    let mut response_stream = resource.ok_or_else(|| {
        JSError::new_typ(&ctx, "Invalid internal resource object").unwrap()
    })?;

    let (promise, resolver) = JSPromise::new_pending(&ctx)?;
    let future = async move {
        let result: Option<Result<Bytes, FetchError>> = response_stream.next().await;
        NativeJob::new(move |ctx| {
            match result {
                Some(Ok(bytes)) => {
                    let mut bytes: ManuallyDrop<Vec<u8>> =
                        ManuallyDrop::new(bytes.to_vec());
                    let chunk = JSTypedArray::with_bytes(
                        ctx,
                        bytes.as_mut_slice(),
                        rust_jsc::JSTypedArrayType::Uint8Array,
                    )?
                    .into();
                    resolver.resolve(None, &[chunk])?;
                }
                Some(Err(err)) => {
                    let err_value = JSError::with_message(ctx, err.message).unwrap();
                    resolver.reject(None, &[err_value.into()])?;
                }
                None => {
                    resolver.resolve(None, &[])?;
                }
            }
            Ok(())
        })
        .set_name("op_read_response_stream")
    };

    state.job_queue().borrow().spawn(Box::pin(future));
    Ok(promise.into())
}
