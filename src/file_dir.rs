use rust_jsc::{
    class::ClassError, constructor, JSClass, JSContext, JSError, JSObject, JSResult,
    JSValue,
};

use crate::class_manager::ClassManager;

pub struct FsDirEntry {
    pub parent_path: String,
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
}

impl FsDirEntry {
    pub fn as_object(&self, ctx: &JSContext) -> JSResult<JSObject> {
        let object = JSObject::new(ctx);

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

impl FsDirEntry {
    pub const CLASS_NAME: &'static str = "DirEntry";

    pub fn init(manaager: &mut ClassManager) -> Result<(), ClassError> {
        let builder = JSClass::builder(Self::CLASS_NAME);
        let class = builder
            .call_as_constructor(Some(Self::constructor))
            .build()?;

        manaager.insert(class);
        Ok(())
    }

    #[constructor]
    fn constructor(
        ctx: JSContext,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        match args.get(0) {
            Some(object) => {
                let object = object.as_object()?;
                let parent_path = object
                    .get_property("parentPath")
                    .unwrap_or(JSValue::null(&ctx));
                let name = object.get_property("name")?;
                let is_dir = object.get_property("isDir")?;
                let is_file = object.get_property("isFile")?;
                let is_symlink = object.get_property("isSymlink")?;
                this.set_property("parentPath", &parent_path, Default::default())
                    .unwrap();
                this.set_property("name", &name, Default::default())
                    .unwrap();
                this.set_property("isDir", &is_dir, Default::default())
                    .unwrap();
                this.set_property("isFile", &is_file, Default::default())
                    .unwrap();
                this.set_property("isSymlink", &is_symlink, Default::default())
                    .unwrap();
                Ok(this.into())
            }
            None => Err(JSError::new_typ(&ctx, "Invalid arguments").unwrap()),
        }
    }
}
