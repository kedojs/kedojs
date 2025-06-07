use rust_jsc::{JSContext, JSResult};

pub trait JSGlobalObject {
    fn init_globals(ctx: &JSContext) -> JSResult<()>;
}

#[macro_export]
macro_rules! define_globals {
    // Variant 1: With scope
    ($struct_name:ident, @scope[$scope:expr], $($method:ident => $func:path),* $(,)?) => {
        impl kedo_utils::JSGlobalObject for $struct_name {
            fn init_globals(ctx: &rust_jsc::JSContext) -> rust_jsc::JSResult<()> {
                let global = ctx.global_object();
                let obj = JSObject::new(ctx);

                static DESCRIPTOR: std::sync::OnceLock<rust_jsc::PropertyDescriptor> = std::sync::OnceLock::new();
                let descriptor = DESCRIPTOR.get_or_init(|| {
                    rust_jsc::PropertyDescriptorBuilder::new()
                        .writable(false)
                        .configurable(true)
                        .enumerable(false)
                        .build()
                });

                $(
                    let func = rust_jsc::JSFunction::callback(ctx, Some(stringify!($method)), Some($func));
                    obj.set_property(stringify!($method), &func, *descriptor)?;
                )*

                global.set_property($scope, &obj, Default::default())?;
                Ok(())
            }
        }
    };

    // Variant 2: Direct global functions
    ($struct_name:ident, $($method:ident => $func:path),* $(,)?) => {
        impl kedo_utils::JSGlobalObject for $struct_name {
            fn init_globals(ctx: &rust_jsc::JSContext) -> rust_jsc::JSResult<()> {
                let global = ctx.global_object();

                static DESCRIPTOR: std::sync::OnceLock<rust_jsc::PropertyDescriptor> = std::sync::OnceLock::new();
                let descriptor = DESCRIPTOR.get_or_init(|| {
                    rust_jsc::PropertyDescriptorBuilder::new()
                        .writable(false)
                        .configurable(true)
                        .enumerable(false)
                        .build()
                });

                $(
                    let func = rust_jsc::JSFunction::callback(ctx, Some(stringify!($method)), Some($func));
                    global.set_property(stringify!($method), &func, *descriptor)?;
                )*

                Ok(())
            }
        }
    };
}
