use std::io;

use rust_jsc::{
    callback, JSArray, JSContext, JSError, JSFunction, JSObject, JSPromise, JSResult,
    JSValue,
};

use crate::{
    async_util::asyncify,
    context::{downcast_state, KedoContext},
    file_dir::FsDirEntry,
    job::{AsyncJobQueue, NativeJob},
};

pub struct StdFileSystem;

impl StdFileSystem {
    pub fn read_file_evt(path: &str) -> io::Result<String> {
        let contents = std::fs::read_to_string(path)?;
        Ok(contents)
    }

    pub async fn read_file_async_evt(path: &str) -> io::Result<String> {
        let path = path.to_owned();
        let contents = asyncify(move || Self::read_file_evt(&path)).await?;
        Ok(contents)
    }

    pub fn write_file_evt(path: &str, data: &str) -> io::Result<()> {
        std::fs::write(path, data)?;
        Ok(())
    }

    pub async fn write_file_async_evt(path: &str, data: &str) -> io::Result<()> {
        let path = path.to_owned();
        let data = data.to_owned();
        asyncify(move || Self::write_file_evt(&path, &data)).await?;
        Ok(())
    }

    pub fn read_dir_evt(path: &str) -> io::Result<Vec<FsDirEntry>> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let is_dir = metadata.is_dir();
            let is_file = metadata.is_file();
            let is_symlink = metadata.file_type().is_symlink();

            let parent_path = entry
                .path()
                .parent()
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, "parent path not found")
                })?
                .to_string_lossy()
                .to_string();
            let name = entry.file_name().to_string_lossy().to_string();

            entries.push(FsDirEntry {
                name,
                parent_path,
                is_dir,
                is_file,
                is_symlink,
            });
        }

        Ok(entries)
    }

    pub async fn read_dir_async_evt(path: &str) -> io::Result<Vec<FsDirEntry>> {
        let path = path.to_owned();
        let entries = asyncify(move || Self::read_dir_evt(&path)).await?;
        Ok(entries)
    }

    /// remove file, directory, or symlink
    pub fn remove_evt(path: &str, recursive: bool) -> io::Result<()> {
        // check type of file
        let metadata = std::fs::metadata(path)?;
        let file_type = metadata.file_type();

        if file_type.is_dir() {
            if recursive {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_dir(path)?;
            }
        } else if metadata.file_type().is_symlink() {
            // support remove of non unix-like system
            #[cfg(not(unix))]
            {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "symlink removal is not supported on this platform",
                ));
            }

            #[cfg(unix)]
            {
                std::fs::remove_file(path)?;
            }
        } else {
            std::fs::remove_file(path)?;
        }

        Ok(())
    }

    pub fn path_separator() -> String {
        std::path::MAIN_SEPARATOR.to_string()
    }

    pub async fn remove_async_evt(path: &str, recursive: bool) -> io::Result<()> {
        let path = path.to_owned();
        asyncify(move || Self::remove_evt(&path, recursive)).await?;
        Ok(())
    }
}

pub struct FileSystem;

impl FileSystem {
    pub fn object(ctx: &rust_jsc::JSContext) -> JSResult<JSObject> {
        let file_system = JSObject::new(ctx);

        let read_sync_callback =
            JSFunction::callback(ctx, Some("readFileSync"), Some(Self::read_file_sync));
        file_system.set_property(
            "readFileSync",
            &read_sync_callback,
            Default::default(),
        )?;

        let write_sync_callback =
            JSFunction::callback(ctx, Some("writeFileSync"), Some(Self::write_file_sync));
        file_system.set_property(
            "writeFileSync",
            &write_sync_callback,
            Default::default(),
        )?;

        let remove_sync_callback =
            JSFunction::callback(ctx, Some("removeSync"), Some(Self::remove_sync));
        file_system.set_property(
            "removeSync",
            &remove_sync_callback,
            Default::default(),
        )?;

        let read_dir_sync_callback =
            JSFunction::callback(ctx, Some("readDirSync"), Some(Self::read_dir_sync));
        file_system.set_property(
            "readDirSync",
            &read_dir_sync_callback,
            Default::default(),
        )?;

        let read_callback =
            JSFunction::callback(ctx, Some("readFile"), Some(Self::read_file));
        file_system.set_property("readFile", &read_callback, Default::default())?;

        let write_callback =
            JSFunction::callback(ctx, Some("writeFile"), Some(Self::write_file));
        file_system.set_property("writeFile", &write_callback, Default::default())?;

        let remove_callback =
            JSFunction::callback(ctx, Some("remove"), Some(Self::remove));
        file_system.set_property("remove", &remove_callback, Default::default())?;

        let read_dir_callback =
            JSFunction::callback(ctx, Some("readDir"), Some(Self::read_dir));
        file_system.set_property("readDir", &read_dir_callback, Default::default())?;

        Ok(file_system)
    }

    #[callback]
    fn read_file_sync(
        ctx: JSContext,
        _: JSObject,
        __: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let path = args[0].as_string()?.to_string();
        let content = StdFileSystem::read_file_evt(&path);
        match content {
            Ok(content) => Ok(JSValue::string(&ctx, content)),
            Err(err) => Err(JSError::with_message(&ctx, format!("{}", err)).unwrap()),
        }
    }

    #[callback]
    fn read_dir_sync(
        ctx: JSContext,
        _: JSObject,
        __: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let path = args[0].as_string()?.to_string();
        let entries = StdFileSystem::read_dir_evt(&path);
        match entries {
            Ok(entries) => {
                let mut values: Vec<JSValue> = Vec::new();
                for (_, dir) in entries.into_iter().enumerate() {
                    values.push(dir.as_object(&ctx)?.into());
                }

                let array = JSArray::new_array(&ctx, values.as_slice())?;
                Ok(array.into())
            }
            Err(err) => Err(JSError::with_message(&ctx, format!("{}", err)).unwrap()),
        }
    }

    #[callback]
    fn write_file_sync(
        ctx: JSContext,
        _: JSObject,
        __: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let path = args[0].as_string()?.to_string();
        let data = args[1].as_string()?.to_string();
        let content = StdFileSystem::write_file_evt(&path, &data);
        match content {
            Ok(_) => Ok(JSValue::undefined(&ctx)),
            Err(err) => Err(JSError::with_message(&ctx, format!("{}", err)).unwrap()),
        }
    }

    #[callback]
    fn remove_sync(
        ctx: JSContext,
        _: JSObject,
        __: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let path = args
            .get(0)
            .ok_or_else(|| JSError::new_typ(&ctx, "missing argument").unwrap())?
            .as_string()?
            .to_string();
        let recursive = args.get(1).map_or(false, |value| value.as_boolean());

        let content = StdFileSystem::remove_evt(&path, recursive);
        match content {
            Ok(_) => Ok(JSValue::undefined(&ctx)),
            Err(err) => Err(JSError::with_message(&ctx, format!("{}", err)).unwrap()),
        }
    }

    #[callback]
    fn read_file(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let path = args[0].as_string()?.to_string();
        let state = KedoContext::from(&ctx).state();

        let (promise, resolver) = JSPromise::new_pending(&ctx)?;
        let future = async move {
            let content = StdFileSystem::read_file_async_evt(&path).await;
            NativeJob::new(move |ctx| {
                match content {
                    Ok(content) => {
                        let content = JSValue::string(ctx, content);
                        resolver.resolve(Some(&this), &[content]).unwrap();
                    }
                    Err(err) => {
                        let err_value =
                            JSError::with_message(ctx, format!("{}", err)).unwrap();
                        resolver.reject(Some(&this), &[err_value.into()]).unwrap();
                    }
                }
                Ok(())
            })
        };

        state.job_queue().borrow().spawn(Box::pin(future));
        Ok(promise.into())
    }

    #[callback]
    fn write_file(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let path = args[0].as_string()?.to_string();
        let data = args[1].as_string()?.to_string();
        let state = KedoContext::from(&ctx).state();

        let (promise, resolver) = JSPromise::new_pending(&ctx)?;
        let future = async move {
            let content = StdFileSystem::write_file_async_evt(&path, &data).await;
            NativeJob::new(move |ctx| {
                match content {
                    Ok(_) => {
                        resolver.resolve(Some(&this), &[]).unwrap();
                    }
                    Err(err) => {
                        let err_value =
                            JSError::with_message(ctx, format!("{}", err)).unwrap();
                        resolver.reject(Some(&this), &[err_value.into()]).unwrap();
                    }
                }
                Ok(())
            })
        };

        state.job_queue().borrow().spawn(Box::pin(future));
        Ok(promise.into())
    }

    #[callback]
    fn read_dir(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let path = args[0].as_string()?.to_string();
        let state = downcast_state::<AsyncJobQueue>(&ctx);

        let (promise, resolver) = JSPromise::new_pending(&ctx)?;
        let future = async move {
            let entries = StdFileSystem::read_dir_async_evt(&path).await;
            NativeJob::new(move |ctx| {
                match entries {
                    Ok(entries) => {
                        let mut values: Vec<JSValue> = Vec::new();
                        for (_, dir) in entries.into_iter().enumerate() {
                            // TOOD: handle error
                            values.push(dir.as_object(ctx).unwrap().into());
                        }

                        let array = JSArray::new_array(ctx, values.as_slice()).unwrap();
                        resolver.resolve(Some(&this), &[array.into()]).unwrap();
                    }
                    Err(err) => {
                        let err_value =
                            JSError::with_message(ctx, format!("{}", err)).unwrap();
                        resolver.reject(Some(&this), &[err_value.into()]).unwrap();
                    }
                }
                Ok(())
            })
        };

        state.job_queue().borrow().spawn(Box::pin(future));
        Ok(promise.into())
    }

    #[callback]
    fn remove(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let path = args[0].as_string()?.to_string();
        let recursive = args[1].as_boolean();
        let state = KedoContext::from(&ctx).state();

        let (promise, resolver) = JSPromise::new_pending(&ctx)?;
        let future = async move {
            let content = StdFileSystem::remove_async_evt(&path, recursive).await;
            NativeJob::new(move |ctx| {
                match content {
                    Ok(_) => {
                        resolver.resolve(Some(&this), &[]).unwrap();
                    }
                    Err(err) => {
                        let err_value =
                            JSError::with_message(ctx, format!("{}", err)).unwrap();
                        resolver.reject(Some(&this), &[err_value.into()]).unwrap();
                    }
                }
                Ok(())
            })
        };

        state.job_queue().borrow().spawn(Box::pin(future));
        Ok(promise.into())
    }
}
