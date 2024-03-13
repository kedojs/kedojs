use boa_engine::class::Class;
use boa_engine::class::ClassBuilder;
use boa_engine::js_string;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::property::PropertyDescriptor;
use boa_engine::Context;
use boa_engine::JsArgs;
use boa_engine::JsData;
use boa_engine::JsNativeError;
use boa_engine::JsObject;
use boa_engine::JsResult;
use boa_engine::JsString;
use boa_engine::JsValue;
use boa_engine::NativeFunction;
use boa_gc::Finalize;
use boa_gc::Trace;


pub struct FsDirEntry {
    pub parent_path: String,
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
}

#[derive(Clone, Debug, Trace, Finalize, JsData)]
pub struct KedoDirEntry {
    pub parent_path: JsString,
    pub name: JsString,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
}

impl KedoDirEntry {
    fn to_string(this: &JsValue, _: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        if let Some(object) = this.as_object() {
            if let Some(dir_entry) = object.downcast_ref::<KedoDirEntry>() {
                let name = dir_entry.name.clone();
                Ok(JsValue::new(name))
            } else {
                Err(JsNativeError::typ()
                    .with_message("'this' is not a DirEntry object")
                    .into())
            }
        } else {
            Err(JsNativeError::typ()
                .with_message("'this' is not a DirEntry object")
                .into())
        }
    }

    fn define_object_property(
        context: &mut Context,
        instance: &JsObject,
        parent_path: JsString,
        name: JsString,
        is_dir: bool,
        is_file: bool,
        is_symlink: bool,
    ) -> JsResult<()> {
        instance.define_property_or_throw(
            js_string!("parentPath"),
            PropertyDescriptor::builder()
                .value(parent_path)
                .writable(false)
                .enumerable(true)
                .configurable(false),
            context,
        )?;

        instance.define_property_or_throw(
            js_string!("name"),
            PropertyDescriptor::builder()
                .value(name)
                .writable(false)
                .enumerable(true)
                .configurable(false),
            context,
        )?;

        instance.define_property_or_throw(
            js_string!("isDir"),
            PropertyDescriptor::builder()
                .value(is_dir)
                .writable(false)
                .enumerable(true)
                .configurable(false),
            context,
        )?;

        instance.define_property_or_throw(
            js_string!("isFile"),
            PropertyDescriptor::builder()
                .value(is_file)
                .writable(false)
                .enumerable(true)
                .configurable(false),
            context,
        )?;

        instance.define_property_or_throw(
            js_string!("isSymlink"),
            PropertyDescriptor::builder()
                .value(is_symlink)
                .writable(false)
                .enumerable(true)
                .configurable(false),
            context,
        )?;

        let to_string_fn = FunctionObjectBuilder::new(
            context.realm(),
            NativeFunction::from_fn_ptr(Self::to_string),
        )
        .name("toString")
        .build();

        // toString method
        instance.define_property_or_throw(
            js_string!("toString"),
            PropertyDescriptor::builder()
                .value(to_string_fn)
                .writable(false)
                .enumerable(false)
                .configurable(false),
            context,
        )?;

        Ok(())
    }

    pub fn to_object(&self, context: &mut Context) -> JsResult<JsObject> {
        let object = JsObject::from_proto_and_data(None, self.clone());
        Self::define_object_property(
            context,
            &object,
            self.parent_path.clone(),
            self.name.clone(),
            self.is_dir,
            self.is_file,
            self.is_symlink,
        )?;
        Ok(object)
    }
}

impl Into<JsValue> for KedoDirEntry {
    fn into(self) -> JsValue {
        JsObject::from_proto_and_data(None, self).into()
    }
}

impl From<&FsDirEntry> for KedoDirEntry {
    fn from(entry: &FsDirEntry) -> Self {
        Self {
            parent_path: js_string!(entry.parent_path.clone()),
            name: js_string!(entry.name.clone()),
            is_dir: entry.is_dir,
            is_file: entry.is_file,
            is_symlink: entry.is_symlink,
        }
    }
}

impl Into<JsObject> for KedoDirEntry {
    fn into(self) -> JsObject {
        JsObject::from_proto_and_data(None, self)
    }
}

impl Class for KedoDirEntry {
    const NAME: &'static str = "KedoDirEntry";
    const LENGTH: usize = 5;

    fn data_constructor(
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<Self> {
        let parent_path = args.get_or_undefined(0).to_string(context)?;
        let name = args.get_or_undefined(1).to_string(context)?;
        let is_dir = args.get_or_undefined(2).to_boolean();
        let is_file = args.get_or_undefined(3).to_boolean();
        let is_symlink = args.get_or_undefined(4).to_boolean();

        Ok(Self {
            parent_path,
            name,
            is_dir,
            is_file,
            is_symlink,
        })
    }

    fn object_constructor(
        instance: &JsObject,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<()> {
        let parent_path = args.get_or_undefined(0).to_string(context)?;
        let name = args.get_or_undefined(1).to_string(context)?;
        let is_dir = args.get_or_undefined(2).to_boolean();
        let is_file = args.get_or_undefined(3).to_boolean();
        let is_symlink = args.get_or_undefined(4).to_boolean();

        Self::define_object_property(
            context,
            instance,
            parent_path,
            name,
            is_dir,
            is_file,
            is_symlink,
        )?;

        Ok(())
    }

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.static_method(
            js_string!("is"),
            1,
            NativeFunction::from_fn_ptr(|_this, args, _ctx| {
                if let Some(arg) = args.first() {
                    if let Some(object) = arg.as_object() {
                        if object.is::<KedoDirEntry>() {
                            return Ok(true.into());
                        }
                    }
                }

                Ok(false.into())
            }),
        );

        Ok(())
    }
}