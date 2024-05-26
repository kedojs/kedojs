use rust_jsc::{
    class::ClassError, constructor, has_instance, JSClass, JSClassAttribute, JSContext,
    JSError, JSObject, JSResult, JSValue, PropertyDescriptorBuilder,
};

use crate::{
    class_table::ClassTable,
    context::{downcast_state, KedoContext},
    job::AsyncJobQueue,
};

pub struct FsDirEntry {
    pub parent_path: String,
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
}

impl FsDirEntry {
    pub fn as_object(&self, ctx: &JSContext) -> JSResult<JSObject> {
        let state = downcast_state::<AsyncJobQueue>(ctx);
        let class = state.classes().get(DirEntry::CLASS_NAME).unwrap();
        let object = class.object::<FsDirEntry>(ctx, None);

        let parent_path = JSValue::string(ctx, self.parent_path.clone());
        object
            .set_property("parentPath", &parent_path, Default::default())
            .unwrap();

        let name = JSValue::string(ctx, self.name.clone());
        object
            .set_property("name", &name, Default::default())
            .unwrap();

        let is_dir = JSValue::boolean(ctx, self.is_dir);
        object
            .set_property("isDir", &is_dir, Default::default())
            .unwrap();

        let is_file = JSValue::boolean(ctx, self.is_file);
        object
            .set_property("isFile", &is_file, Default::default())
            .unwrap();

        let is_symlink = JSValue::boolean(ctx, self.is_symlink);
        object
            .set_property("isSymlink", &is_symlink, Default::default())
            .unwrap();

        Ok(object)
    }
}

pub struct DirEntry {}

impl DirEntry {
    pub const CLASS_NAME: &'static str = "DirEntry";

    pub fn init(
        manaager: &mut ClassTable,
        ctx: &JSContext,
        global: &JSObject,
    ) -> Result<(), ClassError> {
        let builder = JSClass::builder(Self::CLASS_NAME);
        let class = builder
            .call_as_constructor(Some(Self::constructor))
            .has_instance(Some(Self::has_instance))
            .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
            .build()?;

        let template_object = class.object::<FsDirEntry>(ctx, None);
        global
            .set_property(DirEntry::CLASS_NAME, &template_object, Default::default())
            .unwrap();

        manaager.insert(class);
        Ok(())
    }

    pub fn is(ctx: &KedoContext, object: &JSObject) -> JSResult<bool> {
        let state = ctx.state();
        let class = state.classes().get(DirEntry::CLASS_NAME).unwrap();
        object.is_object_of_class(class)
    }

    #[has_instance]
    fn has_instance(
        ctx: JSContext,
        _constructor: JSObject,
        value: JSValue,
    ) -> JSResult<bool> {
        let context = KedoContext::from(&ctx);
        let object = value.as_object()?;
        DirEntry::is(&context, &object)
    }

    #[constructor]
    fn constructor(
        ctx: JSContext,
        _constructor: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        match args.get(0) {
            Some(object) => {
                let argument = object.as_object()?;
                let parent_path = argument
                    .get_property("parentPath")
                    .unwrap_or(JSValue::null(&ctx));
                let name = argument.get_property("name")?;
                let is_dir = argument.get_property("isDir")?;
                let is_file = argument.get_property("isFile")?;
                let is_symlink = argument.get_property("isSymlink")?;

                let context = KedoContext::from(&ctx);
                let state = context.state();
                let class = state.classes().get(DirEntry::CLASS_NAME).unwrap();
                let object = class.object::<FsDirEntry>(&ctx, None);

                let descriptor = PropertyDescriptorBuilder::new()
                    .writable(false)
                    .enumerable(true)
                    .configurable(false)
                    .build();
                object
                    .set_property("parentPath", &parent_path, descriptor)
                    .unwrap();
                object.set_property("name", &name, descriptor).unwrap();
                object.set_property("isDir", &is_dir, descriptor).unwrap();
                object.set_property("isFile", &is_file, descriptor).unwrap();
                object
                    .set_property("isSymlink", &is_symlink, descriptor)
                    .unwrap();
                Ok(object.into())
            }
            None => Err(JSError::new_typ(&ctx, "Invalid arguments").unwrap()),
        }
    }
}
