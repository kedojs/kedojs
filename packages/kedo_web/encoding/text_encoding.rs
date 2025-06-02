use super::text_decoder_inner::InnerTextDecoder;
use encoding_rs::{CoderResult, DecoderResult, Encoding};
use kedo_core::define_exports;
use kedo_utils::downcast_ref;
use rust_jsc::{
    callback, JSArrayBuffer, JSContext, JSError, JSObject, JSResult, JSString, JSValue,
};

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
    input: JSValue,
) -> JSResult<JSValue> {
    JSValue::uft8_encode(&ctx, input)
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
