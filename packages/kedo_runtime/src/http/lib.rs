use crate::http::{
    decoder::StreamDecoder, encoder::StreamEncoder, fetch::errors::FetchError,
};
use bytes::Bytes;
use futures::StreamExt;
use kedo_core::{downcast_state, enqueue_job, native_job};
use kedo_macros::js_class;
use kedo_utils::{downcast_ref, js_error, js_error_typ, js_undefined};
use rust_jsc::{callback, JSContext, JSError, JSObject, JSResult, JSTypedArray, JSValue};
use std::mem::ManuallyDrop;

/// | ---------------------- DecodedStreamResource ---------------------- |
///
/// This class is used to create a JS object that wraps a decoded stream.
/// The object is used to read the decoded stream in chunks.
#[js_class(
    resource = StreamDecoder,
)]
pub struct DecodedStreamResource {}

// impl DecodedStreamResource {
//     pub const CLASS_NAME: &'static str = "DecodedBodyStream";

//     pub fn init_class(manaager: &mut ClassTable) -> Result<(), ClassError> {
//         let builder = JSClass::builder(Self::CLASS_NAME);
//         let class = builder
//             .call_as_constructor(Some(Self::constructor))
//             .set_finalize(Some(Self::finalize))
//             .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
//             .build()?;

//         manaager.insert(class);
//         Ok(())
//     }

//     /// finalize is called when the object is being garbage collected.
//     /// This is the place to clean up any resources that the object may hold.
//     #[finalize]
//     fn finalize(data_ptr: PrivateData) {
//         drop_ptr::<StreamDecoder>(data_ptr);
//     }

//     #[constructor]
//     fn constructor(
//         ctx: JSContext,
//         constructor: JSObject,
//         _args: &[JSValue],
//     ) -> JSResult<JSValue> {
//         let state = downcast_state(&ctx);
//         let class = match state.classes().get(DecodedStreamResource::CLASS_NAME) {
//             Some(class) => class,
//             None => Err(JSError::new_typ(&ctx, "DecodedBodyStream class not found")?)?,
//         };

//         let object = class.object::<StreamDecoder>(&ctx, None);
//         object.set_prototype(&constructor);
//         Ok(object.into())
//     }
// }

#[callback]
pub fn op_read_decoded_stream(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    resource: JSObject,
    callback: JSObject,
) -> JSResult<JSValue> {
    let state = downcast_state(&ctx);
    let resource = downcast_ref::<StreamDecoder>(&resource);
    let mut decoded_stream = match resource {
        Some(resource) => resource,
        None => {
            return Err(js_error_typ!(
                &ctx,
                "[Op:ReadDecoded] Invalid internal resource"
            ))
        }
    };

    enqueue_job!(state, async move {
        let result: Option<Result<Bytes, FetchError>> = decoded_stream.next().await;
        native_job!("op_read_decoded_stream", move |ctx| {
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
                    callback.call(None, &[js_undefined!(&ctx), chunk])?;
                }
                Some(Err(err)) => {
                    let error = js_error!(ctx, err.message);
                    callback.call(None, &[error.into()])?;
                }
                None => {
                    callback.call(None, &[])?;
                }
            }
            Ok(())
        })
    });

    Ok(js_undefined!(&ctx))
}

/// | ---------------------- EncodedStreamResource ---------------------- |
///
/// This class is used to create a JS object that wraps an encoded stream.
/// The object is used to read the encoded stream in chunks.
#[js_class(
    resource = StreamEncoder,
    proto = "EncodedStreamResourcePrototype",
)]
pub struct EncodedStreamResource {}

// impl EncodedStreamResource {
//     pub const CLASS_NAME: &'static str = "EncodedBodyStream";

//     pub fn init_class(manaager: &mut ClassTable) -> Result<(), ClassError> {
//         let builder = JSClass::builder(Self::CLASS_NAME);
//         let class = builder
//             .call_as_constructor(Some(Self::constructor))
//             .set_finalize(Some(Self::finalize))
//             .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
//             .build()?;

//         manaager.insert(class);
//         Ok(())
//     }

//     /// finalize is called when the object is being garbage collected.
//     /// This is the place to clean up any resources that the object may hold.
//      #[finalize]
//      fn finalize(data_ptr: PrivateData) {
//          drop_ptr::<StreamEncoder>(data_ptr);
//      }
// }

// #[constructor]
// fn my_constructor(
//     ctx: JSContext,
//     constructor: JSObject,
//     _args: &[JSValue],
// ) -> JSResult<JSValue> {
//     let state = downcast_state(&ctx);
//     let class = match state.classes().get(EncodedStreamResource::CLASS_NAME) {
//         Some(class) => class,
//         None => Err(JSError::new_typ(&ctx, "EncodedBodyStream class not found")?)?,
//     };

//     println!("class: {:?}", "ME");
//     let object = class.object::<StreamEncoder>(&ctx, None);
//     object.set_prototype(&constructor);
//     Ok(object.into())
// }
