use super::text_decoder_inner::InnerTextDecoder;
use encoding_rs::{CoderResult, DecoderResult, Encoding};
use kedo_core::define_exports;
use kedo_utils::downcast_ref;
use rust_jsc::{
    callback, JSArrayBuffer, JSContext, JSError, JSObject, JSResult, JSString,
    JSTypedArray, JSValue,
};
use std::mem::ManuallyDrop;

#[callback]
pub fn encoding_for_label_no_replacement(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    if args.len() == 0 {
        return Err(JSError::new_typ(&ctx, "Expected 1 argument")?);
    }

    let label = args[0].as_string()?.to_string();
    let encoding = match Encoding::for_label_no_replacement(label.as_bytes()) {
        Some(enc) => enc,
        None => {
            return Err(JSError::new_typ(
                &ctx,
                format!("Unknown encoding label: {:?}", label),
            )?)
        }
    };
    Ok(JSValue::string(&ctx, encoding.name().to_lowercase()))
}

#[callback]
pub fn encoding_decode_utf8_once(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let data_arg = match args.get(0) {
        Some(arg) => arg,
        None => return Err(JSError::new_typ(&ctx, "Missing Buffer argument")?),
    };
    let bytes_data = JSArrayBuffer::from_object(data_arg.as_object()?);
    let bytes = bytes_data.bytes()?;

    let ignore_bom = args
        .get(1)
        .and_then(|arg| Some(arg.as_boolean()))
        .unwrap_or(false);

    let buffer = if !ignore_bom
        && bytes.len() >= 3
        && bytes[0] == 0xef
        && bytes[1] == 0xbb
        && bytes[2] == 0xbf
    {
        &bytes[3..]
    } else {
        bytes
    };

    let string = match JSString::try_from(buffer) {
        Ok(s) => s,
        Err(_) => return Err(JSError::new_typ(&ctx, "Invalid UTF-8 sequence")?),
    };
    let string = JSValue::string(&ctx, string);
    Ok(string)
}

#[callback]
pub fn encoding_decode_once(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let data_arg = match args.get(0) {
        Some(arg) => arg,
        None => return Err(JSError::new_typ(&ctx, "Missing Buffer argument")?),
    };

    let bytes_data = JSArrayBuffer::from_object(data_arg.as_object()?);
    let bytes = bytes_data.bytes()?;

    let label = match args.get(1) {
        Some(arg) => arg.as_string()?.to_string(),
        None => return Err(JSError::new_typ(&ctx, "Missing Label argument")?),
    };
    let fatal = match args.get(2) {
        Some(arg) => arg.as_boolean(),
        None => return Err(JSError::new_typ(&ctx, "Missing Fatal argument")?),
    };
    let ignore_bom = args
        .get(3)
        .and_then(|arg| Some(arg.as_boolean()))
        .unwrap_or(false);

    let encoding = match Encoding::for_label(label.as_bytes()) {
        Some(enc) => enc,
        None => {
            return Err(JSError::new_typ(
                &ctx,
                format!("Invalid encoding label: {:?}", label),
            )?)
        }
    };

    let mut decoder = if ignore_bom {
        encoding.new_decoder_without_bom_handling()
    } else {
        encoding.new_decoder_with_bom_removal()
    };

    let max_buffer_length = match decoder.max_utf16_buffer_length(bytes.len()) {
        Some(len) => len,
        None => return Err(JSError::new_typ(&ctx, "Invalid buffer length")?),
    };
    let mut output = vec![0; max_buffer_length];

    if fatal {
        let (result, _, written) =
            decoder.decode_to_utf16_without_replacement(bytes, &mut output, true);
        match result {
            DecoderResult::InputEmpty => {
                return Ok(JSValue::string(
                    &ctx,
                    String::from_utf16_lossy(&output[..written]),
                ));
            }
            DecoderResult::OutputFull => {
                return Err(JSError::new_typ(&ctx, "Output buffer is too small")?);
            }
            DecoderResult::Malformed(_, _) => {
                return Err(JSError::new_typ(&ctx, "Malformed input")?);
            }
        }
    } else {
        let (result, _, written, _) = decoder.decode_to_utf16(bytes, &mut output, true);
        match result {
            CoderResult::InputEmpty => {
                return Ok(JSValue::string(
                    &ctx,
                    String::from_utf16_lossy(&output[..written]),
                ));
            }
            CoderResult::OutputFull => {
                return Err(JSError::new_typ(&ctx, "Output buffer is too small")?);
            }
        }
    }
}

#[callback]
pub fn encoding_decode(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let decoder_arg = match args.get(0) {
        Some(arg) => arg.as_object()?,
        None => return Err(JSError::new_typ(&ctx, "Missing Decoder argument")?),
    };
    let mut text_decoder = match downcast_ref::<InnerTextDecoder>(&decoder_arg) {
        Some(decoder) => decoder,
        None => return Err(JSError::new_typ(&ctx, "Invalid Decoder object")?),
    };
    let data_arg = match args.get(1) {
        Some(arg) => arg,
        None => return Err(JSError::new_typ(&ctx, "Missing Buffer argument")?),
    };
    let stream = args
        .get(2)
        .and_then(|arg| Some(arg.as_boolean()))
        .unwrap_or(false);
    let bytes_data = JSArrayBuffer::from_object(data_arg.as_object()?);
    let fatal = text_decoder.fatal;
    let bytes = bytes_data.bytes()?;
    let max_buffer_length =
        match text_decoder.decoder.max_utf16_buffer_length(bytes.len()) {
            Some(len) => len,
            None => return Err(JSError::new_typ(&ctx, "Invalid buffer length")?),
        };

    let mut output = vec![0; max_buffer_length];
    if fatal {
        let (result, _, written) = text_decoder
            .decoder
            .decode_to_utf16_without_replacement(bytes, &mut output, !stream);
        match result {
            DecoderResult::InputEmpty => {
                return Ok(JSValue::string(
                    &ctx,
                    String::from_utf16_lossy(&output[..written]),
                ));
            }
            DecoderResult::OutputFull => {
                return Err(JSError::new_typ(&ctx, "Output buffer is too small")?);
            }
            DecoderResult::Malformed(_, _) => {
                return Err(JSError::new_typ(&ctx, "Malformed input")?);
            }
        }
    } else {
        let (result, _, written, _) =
            text_decoder
                .decoder
                .decode_to_utf16(bytes, &mut output, !stream);
        match result {
            CoderResult::InputEmpty => {
                return Ok(JSValue::string(
                    &ctx,
                    String::from_utf16_lossy(&output[..written]),
                ));
            }
            CoderResult::OutputFull => {
                return Err(JSError::new_typ(&ctx, "Output buffer is too small")?);
            }
        }
    }
}

#[callback]
pub fn encoding_encode(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let input = match args.get(0) {
        Some(arg) => arg.as_string()?,
        None => return Err(JSError::new_typ(&ctx, "Missing Input argument")?),
    };

    let mut bytes: ManuallyDrop<Vec<u8>> = ManuallyDrop::new(input.into());
    let output = JSTypedArray::with_bytes::<u8>(
        &ctx,
        &mut bytes,
        rust_jsc::JSTypedArrayType::Uint8Array,
    )?;

    return Ok(output.into());
}

pub struct TextEncodingModule;

define_exports!(
    TextEncodingModule,
    @template[],
    @function[
        encoding_decode,
        encoding_encode,
        encoding_decode_once,
        encoding_decode_utf8_once,
        encoding_for_label_no_replacement
    ]
);

// #[cfg(test)]
// mod tests {

//     use rust_jsc::JSTypedArray;

//     use crate::tests::test_utils::new_runtime;

//     #[test]
//     fn test_encoding_for_label_no_replacement() {
//         let rt = new_runtime();
//         let result = rt.evaluate_module_from_source(
//             r#"
//             import { encoding_for_label_no_replacement } from '@kedo:op/web';
//             globalThis.result = encoding_for_label_no_replacement('utf-8');
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_ok());
//         let result = rt.evaluate_script("globalThis.result", None).unwrap();
//         assert_eq!(result.as_string().unwrap(), "utf-8");
//     }

//     #[test]
//     fn test_encoding_for_label_no_replacement_unknown() {
//         let rt = new_runtime();
//         let result = rt.evaluate_module_from_source(
//             r#"
//             import { encoding_for_label_no_replacement } from '@kedo:op/web';
//             globalThis.result = encoding_for_label_no_replacement('unknown');
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_err());
//     }

//     #[test]
//     fn test_encoding_decode() {
//         let rt = new_runtime();
//         let result = rt.evaluate_module_from_source(
//             r#"
//             import { EncodingTextDecoder, encoding_decode } from '@kedo:op/web';
//             const decoder = new EncodingTextDecoder('utf-8', true, false);
//             globalThis.result = encoding_decode(decoder, new Uint8Array([72, 101, 108, 108, 111]).buffer);
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_ok());
//         let result = rt.evaluate_script("globalThis.result", None).unwrap();
//         assert_eq!(result.as_string().unwrap(), "Hello");
//     }

//     #[test]
//     fn test_encoding_decode_stream() {
//         let rt = new_runtime();
//         let result = rt.evaluate_module_from_source(
//             r#"
//             import { EncodingTextDecoder, encoding_decode } from '@kedo:op/web';
//             const decoder = new EncodingTextDecoder('utf-8', true, false);
//             globalThis.result = encoding_decode(decoder, new Uint8Array([72, 101, 108, 108, 111]).buffer, true);
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_ok());
//         let result = rt.evaluate_script("globalThis.result", None).unwrap();
//         assert_eq!(result.as_string().unwrap(), "Hello");
//     }

//     #[test]
//     fn test_encoding_decode_error() {
//         let rt = new_runtime();
//         let result = rt.evaluate_module_from_source(
//             r#"
//             import { EncodingTextDecoder, encoding_decode } from '@kedo:op/web';
//             const decoder = new EncodingTextDecoder('utf-8', true, false);
//             globalThis.result = encoding_decode(decoder, new Uint8Array([0x80]).buffer);
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_err());
//     }

//     #[test]
//     fn test_encoding_decode_once() {
//         let rt = new_runtime();
//         let result = rt.evaluate_module_from_source(
//             r#"
//             import { EncodingTextDecoder, encoding_decode_once } from '@kedo:op/web';
//             const decoder = new EncodingTextDecoder('utf-8', true, false);
//             globalThis.result = encoding_decode_once(
//                 new Uint8Array([72, 101, 108, 108, 111]).buffer,
//                 'utf-8',
//                 false,
//                 false
//             );
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_ok());
//         let result = rt.evaluate_script("globalThis.result", None).unwrap();
//         assert_eq!(result.as_string().unwrap(), "Hello");
//     }

//     #[test]
//     fn test_encoding_decode_utf8_once() {
//         let rt = new_runtime();
//         let result = rt.evaluate_module_from_source(
//             r#"
//             import { encoding_decode_utf8_once } from '@kedo:op/web';
//             globalThis.result = encoding_decode_utf8_once(new Uint8Array([72, 101, 108, 108, 111]).buffer);
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_ok());
//         let result = rt.evaluate_script("globalThis.result", None).unwrap();
//         assert_eq!(result.as_string().unwrap(), "Hello");
//     }

//     #[test]
//     fn test_encoding_encode() {
//         let rt = new_runtime();
//         let result = rt.evaluate_module_from_source(
//             r#"
//             import { encoding_encode } from '@kedo:op/web';
//             globalThis.result = encoding_encode("Hello");
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_ok());
//         let result = rt.evaluate_script("globalThis.result", None).unwrap();
//         let array = JSTypedArray::from_value(&result).unwrap();
//         let bytes = array.as_vec::<u8>().unwrap();
//         assert_eq!(bytes, &[0x48, 0x65, 0x6c, 0x6c, 0x6f]);
//     }
// }
