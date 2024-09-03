use std::mem::ManuallyDrop;

use encoding_rs::{CoderResult, DecoderResult, Encoding};
use rust_jsc::{
    callback, JSArrayBuffer, JSContext, JSError, JSFunction, JSObject, JSResult,
    JSString, JSTypedArray, JSValue,
};

use crate::utils::downcast_ref;

use super::text_decoder_inner::InnerTextDecoder;

#[callback]
pub fn encoding_for_label_no_replacement(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    if args.len() == 0 {
        return Err(JSError::new_typ(&ctx, "Expected 1 argument").unwrap());
    }

    let label = args[0].as_string().unwrap().to_string();
    let encoding =
        Encoding::for_label_no_replacement(label.as_bytes()).ok_or_else(|| {
            JSError::new_typ(&ctx, format!("Unknown encoding label: {:?}", label))
                .unwrap()
        })?;

    Ok(JSValue::string(&ctx, encoding.name().to_lowercase()))
}

#[callback]
pub fn encoding_decode_utf8_once(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let data_arg = args
        .get(0)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing Buffer argument").unwrap())?;
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

    let string = JSValue::string(&ctx, JSString::from(buffer));
    Ok(string)
}

#[callback]
pub fn encoding_decode_once(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let data_arg = args
        .get(0)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing Buffer argument").unwrap())?;
    let bytes_data = JSArrayBuffer::from_object(data_arg.as_object()?);
    let bytes = bytes_data.bytes()?;

    let label = args
        .get(1)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing Label argument").unwrap())?
        .as_string()?
        .to_string();
    let fatal = args
        .get(2)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing Fatal argument").unwrap())?
        .as_boolean();
    let ignore_bom = args
        .get(3)
        .and_then(|arg| Some(arg.as_boolean()))
        .unwrap_or(false);

    let encoding = Encoding::for_label(label.as_bytes()).ok_or_else(|| {
        JSError::new_typ(&ctx, format!("Invalid encoding label: {}", label)).unwrap()
    })?;
    let mut decoder = if ignore_bom {
        encoding.new_decoder_without_bom_handling()
    } else {
        encoding.new_decoder_with_bom_removal()
    };

    let max_buffer_length = decoder
        .max_utf16_buffer_length(bytes.len())
        .ok_or_else(|| JSError::new_typ(&ctx, "Invalid buffer length").unwrap())?;
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
                return Err(JSError::new_typ(&ctx, "Output buffer is too small").unwrap());
            }
            DecoderResult::Malformed(_, _) => {
                return Err(JSError::new_typ(&ctx, "Malformed input").unwrap());
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
                return Err(JSError::new_typ(&ctx, "Output buffer is too small").unwrap());
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
    let decoder_arg = args
        .get(0)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing Decoder argument").unwrap())?
        .as_object()?;
    let mut text_decoder = downcast_ref::<InnerTextDecoder>(&decoder_arg)
        .ok_or_else(|| JSError::new_typ(&ctx, "Invalid Decoder object").unwrap())?;

    let data_arg = args
        .get(1)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing Buffer argument").unwrap())?;
    let stream = args
        .get(2)
        .and_then(|arg| Some(arg.as_boolean()))
        .unwrap_or(false);
    let bytes_data = JSArrayBuffer::from_object(data_arg.as_object()?);
    let fatal = text_decoder.fatal;
    let bytes = bytes_data.bytes()?;
    let max_buffer_length = text_decoder
        .decoder
        .max_utf16_buffer_length(bytes.len())
        .ok_or_else(|| JSError::new_typ(&ctx, "Invalid buffer length").unwrap())?;

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
                return Err(JSError::new_typ(&ctx, "Output buffer is too small").unwrap());
            }
            DecoderResult::Malformed(_, _) => {
                return Err(JSError::new_typ(&ctx, "Malformed input").unwrap());
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
                return Err(JSError::new_typ(&ctx, "Output buffer is too small").unwrap());
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
    let input = args
        .get(0)
        .ok_or_else(|| JSError::new_typ(&ctx, "Missing Input argument").unwrap())?
        .as_string()?;

    let mut bytes: ManuallyDrop<Vec<u8>> = ManuallyDrop::new(input.into());
    let output = JSTypedArray::with_bytes::<u8>(
        &ctx,
        &mut bytes,
        rust_jsc::JSTypedArrayType::Uint8Array,
    )?;

    return Ok(output.into());
}

pub fn encoding_exports(ctx: &JSContext, exports: &JSObject) {
    let encoding_for_label_no_replacement_fn = JSFunction::callback(
        ctx,
        Some("encoding_for_label_no_replacement"),
        Some(encoding_for_label_no_replacement),
    );

    let encoding_decode_fn =
        JSFunction::callback(ctx, Some("encoding_decode"), Some(encoding_decode));
    let encoding_decode_once_fn = JSFunction::callback(
        ctx,
        Some("encoding_decode_once"),
        Some(encoding_decode_once),
    );
    let encoding_decode_utf8_once_fn = JSFunction::callback(
        ctx,
        Some("encoding_decode_utf8_once"),
        Some(encoding_decode_utf8_once),
    );

    let encoding_encode_fn =
        JSFunction::callback(ctx, Some("encoding_encode"), Some(encoding_encode));

    exports
        .set_property(
            "encoding_for_label_no_replacement",
            &encoding_for_label_no_replacement_fn,
            Default::default(),
        )
        .unwrap();
    exports
        .set_property("encoding_decode", &encoding_decode_fn, Default::default())
        .unwrap();
    exports
        .set_property(
            "encoding_decode_once",
            &encoding_decode_once_fn,
            Default::default(),
        )
        .unwrap();
    exports
        .set_property(
            "encoding_decode_utf8_once",
            &encoding_decode_utf8_once_fn,
            Default::default(),
        )
        .unwrap();
    exports
        .set_property("encoding_encode", &encoding_encode_fn, Default::default())
        .unwrap();
}

#[cfg(test)]
mod tests {

    use rust_jsc::JSTypedArray;

    use crate::tests::test_utils::new_runtime;

    #[test]
    fn test_encoding_for_label_no_replacement() {
        let rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { encoding_for_label_no_replacement } from '@kedo/internal/utils';
            globalThis.result = encoding_for_label_no_replacement('utf-8');
        "#,
            "index.js",
            None,
        );

        assert!(result.is_ok());
        let result = rt.evaluate_script("globalThis.result", None).unwrap();
        assert_eq!(result.as_string().unwrap(), "utf-8");
    }

    #[test]
    fn test_encoding_for_label_no_replacement_unknown() {
        let rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { encoding_for_label_no_replacement } from '@kedo/internal/utils';
            globalThis.result = encoding_for_label_no_replacement('unknown');
        "#,
            "index.js",
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_encoding_decode() {
        let rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { EncodingTextDecoder, encoding_decode } from '@kedo/internal/utils';
            const decoder = new EncodingTextDecoder('utf-8', true, false);
            globalThis.result = encoding_decode(decoder, new Uint8Array([72, 101, 108, 108, 111]).buffer);
        "#,
            "index.js",
            None,
        );

        assert!(result.is_ok());
        let result = rt.evaluate_script("globalThis.result", None).unwrap();
        assert_eq!(result.as_string().unwrap(), "Hello");
    }

    #[test]
    fn test_encoding_decode_stream() {
        let rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { EncodingTextDecoder, encoding_decode } from '@kedo/internal/utils';
            const decoder = new EncodingTextDecoder('utf-8', true, false);
            globalThis.result = encoding_decode(decoder, new Uint8Array([72, 101, 108, 108, 111]).buffer, true);
        "#,
            "index.js",
            None,
        );

        assert!(result.is_ok());
        let result = rt.evaluate_script("globalThis.result", None).unwrap();
        assert_eq!(result.as_string().unwrap(), "Hello");
    }

    #[test]
    fn test_encoding_decode_error() {
        let rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { EncodingTextDecoder, encoding_decode } from '@kedo/internal/utils';
            const decoder = new EncodingTextDecoder('utf-8', true, false);
            globalThis.result = encoding_decode(decoder, new Uint8Array([0x80]).buffer);
        "#,
            "index.js",
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_encoding_decode_once() {
        let rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { EncodingTextDecoder, encoding_decode_once } from '@kedo/internal/utils';
            const decoder = new EncodingTextDecoder('utf-8', true, false);
            globalThis.result = encoding_decode_once(
                new Uint8Array([72, 101, 108, 108, 111]).buffer,
                'utf-8',
                false,
                false
            );
        "#,
            "index.js",
            None,
        );

        assert!(result.is_ok());
        let result = rt.evaluate_script("globalThis.result", None).unwrap();
        assert_eq!(result.as_string().unwrap(), "Hello");
    }

    #[test]
    fn test_encoding_decode_utf8_once() {
        let rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { encoding_decode_utf8_once } from '@kedo/internal/utils';
            globalThis.result = encoding_decode_utf8_once(new Uint8Array([72, 101, 108, 108, 111]).buffer);
        "#,
            "index.js",
            None,
        );

        assert!(result.is_ok());
        let result = rt.evaluate_script("globalThis.result", None).unwrap();
        assert_eq!(result.as_string().unwrap(), "Hello");
    }

    #[test]
    fn test_encoding_encode() {
        let rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { encoding_encode } from '@kedo/internal/utils';
            globalThis.result = encoding_encode("Hello");
        "#,
            "index.js",
            None,
        );

        assert!(result.is_ok());
        let result = rt.evaluate_script("globalThis.result", None).unwrap();
        let array = JSTypedArray::from_value(&result).unwrap();
        let bytes = array.as_vec::<u8>().unwrap();
        assert_eq!(bytes, &[0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    }
}
