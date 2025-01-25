use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, DeriveInput, LitStr, Path,
    Result, Token,
};

struct JsClassArgs {
    resource: Path,
    proto: Option<LitStr>,
    constructor: Option<syn::Ident>, // Changed from Path to Ident
    finalizer: Option<syn::Ident>,   // Changed from Path to Ident
}

impl Parse for JsClassArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut resource = None;
        let mut proto = None;
        let mut constructor = None;
        let mut finalizer = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "resource" => resource = Some(input.parse()?),
                "proto" => proto = Some(input.parse()?),
                "constructor" => constructor = Some(input.parse()?),
                "finalizer" => finalizer = Some(input.parse()?),
                _ => return Err(input.error(format!("unknown attribute: {}", ident))),
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(JsClassArgs {
            resource: resource.ok_or_else(|| input.error("resource type required"))?,
            proto,
            constructor,
            finalizer,
        })
    }
}

#[proc_macro_attribute]
pub fn js_class(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as JsClassArgs);
    let input = parse_macro_input!(item as DeriveInput);

    // Find existing impl block and custom implementations
    let has_constructor = args.constructor.is_some();
    let has_finalizer = args.finalizer.is_some();

    let struct_name = &input.ident;
    let resource_type = &args.resource;

    let constructor_name = args.constructor.unwrap_or_else(|| {
        let ident = format_ident!("constructor");
        return ident;
    });
    let finalizer_name = args.finalizer.unwrap_or_else(|| {
        let ident = format_ident!("finalizer");
        return ident;
    });

    // Generate implementations
    let constructor_impl = if !has_constructor {
        quote! {
            #[rust_jsc::constructor]
            fn #constructor_name(
                ctx: rust_jsc::JSContext,
                constructor: rust_jsc::JSObject,
                _args: &[rust_jsc::JSValue]
            ) -> rust_jsc::JSResult<rust_jsc::JSValue> {
                let state = kedo_core::downcast_state(&ctx);
                let class = match state.classes().get(#struct_name::CLASS_NAME) {
                    Some(class) => class,
                    None => return Err(rust_jsc::JSError::new_typ(
                        &ctx,
                        format!("{} class not found", #struct_name::CLASS_NAME),
                    )?),
                };

                let object = class.object::<#resource_type>(&ctx, None);
                object.set_prototype(&constructor);
                Ok(object.into())
            }
        }
    } else {
        quote! {}
    };
    // Generate finalizer implementation
    let finalizer_impl = if !has_finalizer {
        quote! {
            #[rust_jsc::finalize]
            fn #finalizer_name(data_ptr: rust_jsc::PrivateData) {
                kedo_utils::drop_ptr::<#resource_type>(data_ptr);
            }
        }
    } else {
        quote! {}
    };

    let proto_impl = if let Some(proto_name) = args.proto {
        quote! {
            pub const PROTO_NAME: &'static str = #proto_name;

            pub fn init_proto(
                proto_manager: &mut kedo_core::ProtoTable,
                manager: &mut kedo_core::ClassTable,
                ctx: &rust_jsc::JSContext,
            ) -> Result<(), rust_jsc::class::ClassError> {
                let class = manager.get(Self::CLASS_NAME)
                    .ok_or_else(|| rust_jsc::class::ClassError::RetainFailed)?;
                let template_object = class.object::<#resource_type>(ctx, None);
                proto_manager.insert(Self::PROTO_NAME.to_string(), template_object);
                Ok(())
            }

            pub fn template_object(ctx: &rust_jsc::JSContext, scope: &rust_jsc::JSObject) -> rust_jsc::JSResult<()> {
                let state = kedo_core::downcast_state(ctx);
                let template_object = match state.protos().get(Self::PROTO_NAME) {
                        Some(template_object) => template_object,
                        None => return Err(rust_jsc::JSError::new_typ(
                            ctx,
                            format!("{} prototype not found", Self::PROTO_NAME),
                        )?),
                    };
                scope.set_property(Self::CLASS_NAME, &template_object, Default::default())?;
                Ok(())
            }
        }
    } else {
        quote! {}
    };

    // Generate the final path for contructor wiht the struct name
    // if the constructor is not provided, otherwise use the provided path
    let path_constructor = if !has_constructor {
        quote! { #struct_name::#constructor_name }
    } else {
        quote! { #constructor_name }
    };
    // Generate the final path for finalizer wiht the struct name
    let path_finalizer = if !has_finalizer {
        quote! { #struct_name::#finalizer_name }
    } else {
        quote! { #finalizer_name }
    };

    let expanded = quote! {
        pub struct #struct_name {}

        impl #struct_name {
            pub const CLASS_NAME: &'static str = stringify!(#struct_name);

            #finalizer_impl

            #constructor_impl

            pub fn init_class(manager: &mut kedo_core::ClassTable) -> Result<(), rust_jsc::class::ClassError> {
                let builder = rust_jsc::JSClass::builder(Self::CLASS_NAME)
                    .call_as_constructor(Some(#path_constructor))
                    .set_finalize(Some(#path_finalizer))
                    .set_attributes(rust_jsc::JSClassAttribute::NoAutomaticPrototype.into());

                let class = builder.build()?;
                manager.insert(class);
                Ok(())
            }

            #proto_impl
        }
    };

    TokenStream::from(expanded)
}
