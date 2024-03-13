use boa_engine::job::NativeJob;
use boa_engine::js_string;
use boa_engine::object::builtins::JsArray;
use boa_engine::object::builtins::JsPromise;
use boa_engine::object::ObjectInitializer;
use boa_engine::Context;
use boa_engine::JsData;
use boa_engine::JsError;
use boa_engine::JsNativeError;
use boa_engine::JsObject;
use boa_engine::JsResult;
use boa_engine::JsValue;
use boa_engine::NativeFunction;
use boa_gc::Finalize;
use boa_gc::Trace;
use std::future::Future;
use std::io;
use std::sync::Arc;

use crate::file_dir::{FsDirEntry, KedoDirEntry};
use crate::util::asyncify;

// read file with std::fs
fn read_file_evt(path: &str) -> io::Result<String> {
    let contents = std::fs::read_to_string(path)?;
    Ok(contents)
}

async fn read_file_async_evt(path: &str) -> io::Result<String> {
    let path = path.to_owned();
    let contents = asyncify(move || read_file_evt(&path)).await?;
    Ok(contents)
}

fn write_file_evt(path: &str, data: &str) -> io::Result<()> {
    std::fs::write(path, data)?;
    Ok(())
}

async fn write_file_async_evt(path: &str, data: &str) -> io::Result<()> {
    let path = path.to_owned();
    let data = data.to_owned();
    asyncify(move || write_file_evt(&path, &data)).await?;
    Ok(())
}

fn read_dir_evt(path: &str) -> io::Result<Vec<FsDirEntry>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let is_dir = metadata.is_dir();
        let is_file = metadata.is_file();
        let is_symlink = metadata.file_type().is_symlink();

        let parent_path = entry.path().parent().unwrap().to_string_lossy().to_string();
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

async fn read_dir_async_evt(path: &str) -> io::Result<Vec<FsDirEntry>> {
    let path = path.to_owned();
    let entries = asyncify(move || read_dir_evt(&path)).await?;
    Ok(entries)
}

/// remove file, directory, or symlink
fn remove_evt(path: &str, recursive: bool) -> io::Result<()> {
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

async fn remove_async_evt(path: &str, recursive: bool) -> io::Result<()> {
    let path = path.to_owned();
    asyncify(move || remove_evt(&path, recursive)).await?;
    Ok(())
}

// Struct for asynchronous file operations
#[derive(Debug, Default, Trace, Finalize, JsData)]
pub struct FileSystem;

impl FileSystem {
    /// Initializes the `FileSystem` object
    ///
    /// `FileSystem` is a built-in object that provides a way to interact with the file system
    pub fn init(context: &mut Context) -> JsObject {
        fn file_method_async<Fut: std::future::IntoFuture<Output = JsResult<JsValue>> + 'static>(
            f: fn(&JsValue, &[JsValue], &mut Context) -> Fut,
        ) -> NativeFunction {
            // SAFETY: `File` doesn't contain types that need tracing.
            unsafe {
                NativeFunction::from_closure(move |this, args, context| {
                    let future = f(this, args, context);
                    Ok(JsPromise::from_future(future, context).into())
                })
            }
        }

        fn file_method_promise(
            f: fn(&JsValue, &[JsValue], &mut Context) -> JsPromise,
        ) -> NativeFunction {
            // SAFETY: `File` doesn't contain types that need tracing.
            unsafe {
                NativeFunction::from_closure(move |this, args, context| {
                    Ok(f(this, args, context).into())
                })
            }
        }

        ObjectInitializer::with_native_data(Self::default(), context)
            .function(
                NativeFunction::from_fn_ptr(Self::read_file_sync),
                js_string!("readFileSync"),
                0,
            )
            .function(
                NativeFunction::from_fn_ptr(Self::write_file_sync),
                js_string!("writeFileSync"),
                0,
            )
            .function(
                NativeFunction::from_fn_ptr(Self::read_dir_sync),
                js_string!("readDirSync"),
                0,
            )
            .function(
                NativeFunction::from_fn_ptr(Self::remove_sync),
                js_string!("removeSync"),
                0,
            )
            .function(
                file_method_async(Self::read_file),
                js_string!("readFile"),
                0,
            )
            .function(
                file_method_async(Self::write_file),
                js_string!("writeFile"),
                0,
            )
            .function(
                file_method_promise(Self::read_dir),
                js_string!("readDir"),
                0,
            )
            .function(file_method_promise(Self::remove), js_string!("remove"), 0)
            .build()
    }

    /// `Kedo.readFileSync(path: string): string`
    ///
    /// Synchronously reads the entire contents of a file. This is a blocking operation
    /// Returns a string containing the entire contents of the file
    fn read_file_sync(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let path = args
            .get(0)
            .expect("No path argument provided")
            .to_string(context)?
            .to_std_string_escaped();

        let contents = match read_file_evt(&path) {
            Ok(contents) => js_string!(contents),
            Err(e) => return Err(JsNativeError::typ().with_message(e.to_string()).into()),
        };

        Ok(JsValue::new(contents))
    }

    /// `Kedo.writeFileSync(path: string, data: string): void`
    ///
    /// Synchronously writes data to a file, replacing the file if it already exists
    /// If the file does not exist, it will be created
    ///
    /// `path` - The path to the file
    /// `data` - The data to write to the file
    ///
    /// Returns `undefined`
    fn write_file_sync(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let path = args
            .get(0)
            .expect("No path argument provided")
            .to_string(context)?
            .to_std_string_escaped();

        let data = args
            .get(1)
            .unwrap_or(&JsValue::String(js_string!("")))
            .to_string(context)?
            .to_std_string_escaped();

        let result = write_file_evt(&path, &data);
        match result {
            Ok(_) => Ok(JsValue::undefined()),
            Err(e) => Err(JsNativeError::typ().with_message(e.to_string()).into()),
        }
    }

    /// `Kedo.readFile(path: string): Promise<string>`
    ///
    /// Asynchronously reads the entire contents of a file
    ///
    /// `path` - The path to the file
    ///
    /// Returns a promise that resolves with a string containing the entire contents of the file
    ///
    /// Example:
    /// ```javascript
    ///  const { readFile } = Kedo;
    ///  readFile('file.txt').then((data) => {
    ///   console.log(data);
    /// });
    /// ```
    fn read_file(
        _: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> {
        let path = args
            .get(0)
            .expect("No path argument provided")
            .to_string(context)
            .unwrap()
            .to_std_string_escaped();

        let path = Arc::new(path); // Wrap the path in an Arc

        async move {
            let result = read_file_async_evt(&path).await;
            match result {
                Ok(contents) => Ok(JsValue::new(js_string!(contents))),
                Err(e) => Err(JsNativeError::typ().with_message(e.to_string()).into()),
            }
        }
    }

    /// `Kedo.writeFile(path: string, data: string): Promise<void>`
    ///
    /// Asynchronously writes data to a file, replacing the file if it already exists
    /// If the file does not exist, it will be created
    /// `path` - The path to the file
    /// `data` - The data to write to the file
    /// Returns a promise that resolves with `undefined` upon success
    /// Example:
    /// ```javascript
    /// const { writeFile } = Kedo;
    /// writeFile('file.txt', 'Hello, World!').then(() => {
    ///  console.log('File written successfully');
    /// });
    /// ```
    fn write_file(
        _: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> {
        let path = args
            .get(0)
            .expect("No path argument provided")
            .to_string(context)
            .unwrap()
            .to_std_string_escaped();

        let data = args
            .get(1)
            .unwrap_or(&JsValue::String(js_string!("")))
            .to_string(context)
            .unwrap()
            .to_std_string_escaped();

        // let path = Arc::new(path);
        // let data = Arc::new(data);

        async move {
            let result = write_file_async_evt(&path, &data).await;
            match result {
                Ok(_) => Ok(JsValue::undefined()),
                Err(e) => Err(JsNativeError::typ().with_message(e.to_string()).into()),
            }
        }
    }

    /// `Kedo.readDirSync(path: string): KedoDirEntry[]`
    /// Synchronously reads the contents of a directory
    /// `path` - The path to the directory
    ///
    /// Returns an array of `KedoDirEntry` objects
    fn read_dir_sync(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let path = args
            .get(0)
            .expect("No path argument provided")
            .to_string(context)?
            .to_std_string_escaped();

        let entries = match read_dir_evt(&path) {
            Ok(entries) => {
                let entries: Vec<JsValue> = entries
                    .iter()
                    // .map(|entry| entry)
                    .map(|entry| KedoDirEntry::from(entry).to_object(context).unwrap().into())
                    .collect();
                let array = JsArray::from_iter(entries, context);
                JsValue::new(array)
            }
            Err(e) => return Err(JsNativeError::typ().with_message(e.to_string()).into()),
        };

        Ok(entries)
    }

    /// `Kedo.readDir(path: string): Promise<KedoDirEntry[]>`
    ///
    /// Asynchronously reads the contents of a directory
    ///
    /// `path` - The path to the directory
    ///
    /// Returns a promise that resolves with an array of `KedoDirEntry` objects
    fn read_dir(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsPromise {
        let path = args
            .get(0)
            .expect("No path argument provided")
            .to_string(context)
            .unwrap()
            .to_std_string_escaped();

        let path = Arc::new(path);
        let (promise, resolvers) = JsPromise::new_pending(context);

        let future = async move {
            let entries: Result<Vec<FsDirEntry>, JsError> = match read_dir_async_evt(&path).await {
                Ok(dir_entries) => Ok(dir_entries),
                Err(e) => Err(JsNativeError::typ().with_message(e.to_string()).into()),
            };

            NativeJob::new(move |context| match entries {
                Ok(entries) => {
                    let entries: Vec<JsValue> = entries
                        .iter()
                        .map(|entry| KedoDirEntry::from(entry).to_object(context).unwrap().into())
                        .collect();
                    let array = JsArray::from_iter(entries, context);

                    resolvers
                        .resolve
                        .call(&JsValue::undefined(), &[JsValue::new(array)], context)
                }
                Err(e) => {
                    let e = e.to_opaque(context);
                    resolvers.reject.call(&JsValue::undefined(), &[e], context)
                }
            })
        };

        context
            .job_queue()
            .enqueue_future_job(Box::pin(future), context);

        promise
    }

    /// `Kedo.removeSync(path: string, recursive: boolean): void`
    /// Synchronously removes a file, directory, or symlink
    ///
    /// `path` - The path to the file, directory, or symlink
    /// `recursive` - If `true`, removes directories and their contents
    fn remove_sync(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let path = args
            .get(0)
            .expect("No path argument provided")
            .to_string(context)?
            .to_std_string_escaped();

        let recursive = args.get(1).unwrap_or(&JsValue::Boolean(false)).to_boolean();

        let result = remove_evt(&path, recursive);
        match result {
            Ok(_) => Ok(JsValue::undefined()),
            Err(e) => Err(JsNativeError::typ().with_message(e.to_string()).into()),
        }
    }

    /// `Kedo.remove(path: string, recursive: boolean): Promise<void>`
    /// Asynchronously removes a file, directory, or symlink
    ///
    /// `path` - The path to the file, directory, or symlink
    /// `recursive` - If `true`, removes directories and their contents
    fn remove(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsPromise {
        let path = args
            .get(0)
            .expect("No path argument provided")
            .to_string(context)
            .unwrap()
            .to_std_string_escaped();

        let recursive = args.get(1).unwrap_or(&JsValue::Boolean(false)).to_boolean();

        let (promise, resolvers) = JsPromise::new_pending(context);

        let future = async move {
            let result: Result<(), JsError> = match remove_async_evt(&path, recursive).await {
                Ok(_) => Ok(()),
                Err(e) => Err(JsNativeError::typ().with_message(e.to_string()).into()),
            };

            NativeJob::new(move |context| match result {
                Ok(_) => resolvers.resolve.call(&JsValue::undefined(), &[], context),
                Err(e) => {
                    let e = e.to_opaque(context);
                    resolvers.reject.call(&JsValue::undefined(), &[e], context)
                }
            })
        };

        context
            .job_queue()
            .enqueue_future_job(Box::pin(future), context);

        promise
    }
}
