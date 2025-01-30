use kedo_core::{define_exports, downcast_state, enqueue_job, native_job, ModuleSource};
use kedo_utils::{js_error, js_undefined};
use rust_jsc::{callback, JSArray, JSContext, JSError, JSObject, JSResult, JSValue};

use crate::std::StdFileSystem;

pub struct FileSystemModule;

define_exports!(
    FileSystemModule,
    @template[],
    @function[
        op_fs_read_file_sync,
        op_fs_read_dir_sync,
        op_fs_write_file_sync,
        op_fs_remove_sync,
        op_fs_read_file,
        op_fs_remove,
        op_fs_read_dir,
        op_fs_write_file
    ]
);

pub struct FileSystemModuleLoader;

impl ModuleSource for FileSystemModuleLoader {
    fn evaluate(&self, ctx: &JSContext, _name: &str) -> JSObject {
        let exports = JSObject::new(ctx);
        FileSystemModule::export(ctx, &exports)
            .expect("Failed to export FileSystemModule");
        exports
    }

    fn name(&self) -> &str {
        "@kedo:op/fs"
    }
}

#[callback]
fn op_fs_read_file_sync(
    ctx: JSContext,
    _: JSObject,
    __: JSObject,
    path: String,
) -> JSResult<JSValue> {
    let content = StdFileSystem::read_file_evt(&path);
    match content {
        Ok(content) => Ok(JSValue::string(&ctx, content)),
        Err(err) => Err(js_error!(&ctx, format!("{}", err))),
    }
}

#[callback]
fn op_fs_read_dir_sync(
    ctx: JSContext,
    _: JSObject,
    __: JSObject,
    path: String,
) -> JSResult<JSValue> {
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
        Err(err) => Err(js_error!(&ctx, format!("{}", err))),
    }
}

#[callback]
fn op_fs_write_file_sync(
    ctx: JSContext,
    _: JSObject,
    __: JSObject,
    path: String,
    data: String,
) -> JSResult<JSValue> {
    let content = StdFileSystem::write_file_evt(&path, &data);
    match content {
        Ok(_) => Ok(JSValue::undefined(&ctx)),
        Err(err) => Err(js_error!(&ctx, format!("{}", err))),
    }
}

#[callback]
fn op_fs_remove_sync(
    ctx: JSContext,
    _: JSObject,
    __: JSObject,
    path: String,
    recursive: bool,
) -> JSResult<JSValue> {
    let content = StdFileSystem::remove_evt(&path, recursive);
    match content {
        Ok(_) => Ok(JSValue::undefined(&ctx)),
        Err(err) => Err(js_error!(&ctx, format!("{}", err))),
    }
}

#[callback]
fn op_fs_read_file(
    ctx: JSContext,
    _: JSObject,
    __: JSObject,
    path: String,
    callbak: JSObject,
) -> JSResult<JSValue> {
    callbak.protect();

    let state = downcast_state(&ctx);
    enqueue_job!(state, async move {
        let content = StdFileSystem::read_file_async_evt(&path).await;
        native_job!("FileSystem::read_file", move |ctx| {
            match content {
                Ok(content) => {
                    let content = JSValue::string(ctx, content);
                    callbak.call(None, &[js_undefined!(&ctx), content])?;
                }
                Err(err) => {
                    let error = js_error!(ctx, format!("{}", err));
                    callbak.call(None, &[error.into()])?;
                }
            }

            callbak.unprotect();
            Ok(())
        })
    });

    Ok(JSValue::undefined(&ctx))
}

#[callback]
fn op_fs_remove(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    path: String,
    recursive: bool,
    callback: JSObject,
) -> JSResult<JSValue> {
    callback.protect();

    let state = downcast_state(&ctx);
    enqueue_job!(state, async move {
        let content = StdFileSystem::remove_async_evt(&path, recursive).await;
        native_job!("FileSystem::remove", move |ctx| {
            match content {
                Ok(_) => callback.call(None, &[js_undefined!(ctx)])?,
                Err(err) => {
                    let error = js_error!(ctx, format!("{}", err));
                    callback.call(None, &[error.into()])?
                }
            };

            callback.unprotect();
            Ok(())
        })
    });

    Ok(js_undefined!(&ctx))
}

#[callback]
fn op_fs_read_dir(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    path: String,
    callback: JSObject,
) -> JSResult<JSValue> {
    callback.protect();

    let state = downcast_state(&ctx);
    enqueue_job!(state, async move {
        let entries = StdFileSystem::read_dir_async_evt(&path).await;
        native_job!("FileSystem::read_dir", move |ctx| {
            match entries {
                Ok(entries) => {
                    let mut values: Vec<JSValue> = Vec::new();
                    for (_, dir) in entries.into_iter().enumerate() {
                        values.push(dir.as_object(ctx)?.into());
                    }

                    let array = JSArray::new_array(ctx, values.as_slice())?;
                    callback.call(None, &[js_undefined!(ctx), array.into()])?;
                }
                Err(err) => {
                    let error = js_error!(ctx, format!("{}", err));
                    callback.call(None, &[error.into()])?;
                }
            }

            callback.unprotect();
            Ok(())
        })
    });

    Ok(js_undefined!(&ctx))
}

#[callback]
fn op_fs_write_file(
    ctx: JSContext,
    _func: JSObject,
    _this: JSObject,
    path: String,
    data: String,
    callback: JSObject,
) -> JSResult<JSValue> {
    callback.protect();

    let state = downcast_state(&ctx);
    enqueue_job!(state, async move {
        let content = StdFileSystem::write_file_async_evt(&path, &data).await;
        native_job!("FileSystem::write_file", move |ctx| {
            match content {
                Ok(_) => callback.call(None, &[js_undefined!(ctx)])?,
                Err(err) => {
                    let error = js_error!(ctx, format!("{}", err));
                    callback.call(None, &[error.into()])?
                }
            };

            callback.unprotect();
            Ok(())
        })
    });

    Ok(JSValue::undefined(&ctx))
}
