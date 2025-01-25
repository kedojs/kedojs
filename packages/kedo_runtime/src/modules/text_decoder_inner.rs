use encoding_rs::Decoder;
use encoding_rs::Encoding;

use kedo_core::{downcast_state, ClassTable, ProtoTable};
use kedo_utils::drop_ptr;
use rust_jsc::{
    class::ClassError, constructor, finalize, JSClass, JSClassAttribute, JSContext,
    JSError, JSObject, JSResult, JSValue, PrivateData,
};

pub struct InnerTextDecoder {
    pub decoder: Decoder,
    pub fatal: bool,
    #[allow(unused)]
    pub ignore_bom: bool,
}

impl InnerTextDecoder {
    pub fn new(decoder: Decoder, fatal: bool, ignore_bom: bool) -> Self {
        Self {
            decoder,
            fatal,
            ignore_bom,
        }
    }
}

pub struct EncodingTextDecoder {}

impl EncodingTextDecoder {
    pub const CLASS_NAME: &'static str = "EncodingTextDecoder";
    pub const PROTO_NAME: &'static str = "EncodingTextDecoderPrototype";

    pub fn init_proto(
        proto_manager: &mut ProtoTable,
        manager: &mut ClassTable,
        ctx: &JSContext,
    ) -> Result<(), ClassError> {
        ClassError::RetainFailed;
        let class = manager.get(EncodingTextDecoder::CLASS_NAME).unwrap();
        let template_object = class.object::<InnerTextDecoder>(ctx, None);
        proto_manager
            .insert(EncodingTextDecoder::PROTO_NAME.to_string(), template_object);
        Ok(())
    }

    pub fn template_object(ctx: &JSContext, scope: &JSObject) -> JSResult<()> {
        let state = downcast_state(ctx);
        let template_object =
            state.protos().get(EncodingTextDecoder::PROTO_NAME).unwrap();
        scope.set_property(
            EncodingTextDecoder::CLASS_NAME,
            &template_object,
            Default::default(),
        )?;
        Ok(())
    }

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
        drop_ptr::<InnerTextDecoder>(data_ptr);
    }

    #[constructor]
    fn constructor(
        ctx: JSContext,
        constructor: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let label = args
            .get(0)
            .ok_or_else(|| JSError::new_typ(&ctx, "Missing Label argument").unwrap())?
            .as_string()?
            .to_string();
        let fatal = args
            .get(1)
            .ok_or_else(|| JSError::new_typ(&ctx, "Missing Fatal argument").unwrap())?
            .as_boolean();
        let ignore_bom = args
            .get(2)
            .and_then(|arg| Some(arg.as_boolean()))
            .unwrap_or(false);

        let state = downcast_state(&ctx);
        let class = state
            .classes()
            .get(EncodingTextDecoder::CLASS_NAME)
            .unwrap();

        let encoding = Encoding::for_label(label.as_bytes()).ok_or_else(|| {
            JSError::new_typ(&ctx, format!("Invalid encoding label: {}", label)).unwrap()
        })?;
        let decoder = if ignore_bom {
            encoding.new_decoder_without_bom_handling()
        } else {
            encoding.new_decoder_with_bom_removal()
        };

        let inner_decoder = InnerTextDecoder::new(decoder, fatal, ignore_bom);
        let object =
            class.object::<InnerTextDecoder>(&ctx, Some(Box::new(inner_decoder)));
        object.set_prototype(&constructor);
        Ok(object.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::test_utils::new_runtime;

    #[test]
    fn test_text_decoder() {
        let rt = new_runtime();
        let result = rt.evaluate_module_from_source(
            r#"
            import { EncodingTextDecoder } from '@kedo/internal/utils';
            globalThis.encoder = new EncodingTextDecoder('utf-8', true, false);
        "#,
            "index.js",
            None,
        );
        assert!(result.is_ok());
        let result = rt.evaluate_script("globalThis.encoder", None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_object());
    }
}
