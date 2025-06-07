use crate::{
    http::response::FetchResponseExt,
    signals::{InternalSignal, OneshotSignal},
    HttpRequestResource,
};
use futures::future::poll_fn;
use kedo_core::{downcast_state, enqueue_job, native_job, ClassTable, NativeJob};
use kedo_macros::js_class;
use kedo_std::{
    BufferChannelReader, HttpConfig, HttpRequest, HttpRequestEvent, HttpResponse,
    HttpResponseChannel, HttpServer, HttpServerBuilder, HttpSocketAddr, StreamError,
    UnboundedBufferChannelReader,
};
use kedo_utils::{downcast_ref, js_error, js_error_typ, js_undefined};
use rust_jsc::{
    callback, class::ClassError, constructor, finalize, JSClass, JSClassAttribute,
    JSContext, JSError, JSFunction, JSObject, JSResult, JSValue, PrivateData,
};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    vec,
};

struct ServerOptions {
    port: u16,
    hostname: String,
    address: SocketAddr,
    key: Option<String>,
    cert: Option<String>,
}

impl ServerOptions {
    fn from_value(value: &JSValue, ctx: &JSContext) -> JSResult<Self> {
        let value = value.as_object()?;
        let port = value.get_property("port")?.as_number()? as u16;
        let hostname = value.get_property("hostname")?.as_string()?.to_string();
        let key = value
            .get_property("key")
            .and_then(|v| v.as_string())
            .ok()
            .map(|s| s.to_string());
        let cert = value
            .get_property("cert")
            .and_then(|v| v.as_string())
            .ok()
            .map(|s| s.to_string());

        let mut parsed_hostname = match (hostname.clone(), port).to_socket_addrs() {
            Ok(parsed) => parsed,
            Err(err) => {
                return Err(JSError::new_typ(
                    ctx,
                    format!("Failed to parse hostname: {}", err),
                )?);
            }
        };
        let address = match parsed_hostname.next() {
            Some(addr) => addr,
            None => return Err(JSError::new_typ(ctx, "Failed to parse hostname")?),
        };

        Ok(Self {
            port,
            hostname,
            address,
            key,
            cert,
            // handler,
        })
    }

    fn address(&self) -> SocketAddr {
        self.address
    }
}

/// | ---------------------- UnboundedBufferChannelReader ---------------------- |
///
/// This class is used to create a JS object that wraps a channel with the upcoming
/// request events.
#[js_class(
    resource = UnboundedBufferChannelReader<HttpRequestEvent>,
    proto = "NetworkBufferChannelReaderPrototype",
)]
pub struct NetworkBufferChannelReaderResource {}

#[callback]
fn op_read_request_event(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    reader: JSObject,
) -> JSResult<JSValue> {
    let state = downcast_state(&ctx);
    let reader = downcast_ref::<UnboundedBufferChannelReader<HttpRequestEvent>>(&reader);
    let event = match reader {
        Some(mut reader) => reader.try_read(),
        None => {
            return Err(js_error_typ!(
                &ctx,
                "[Op:ReadRequestEvent] Invalid resource object"
            ))
        }
    };

    let mut event = match event {
        Ok(event) => event,
        Err(err) => match err {
            StreamError::Closed | StreamError::Empty => {
                return Ok(JSValue::number(&ctx, err.into()));
            }
            _ => {
                return Err(js_error_typ!(
                    &ctx,
                    format!("[Op:ReadRequestEvent] {}", err)
                ))
            }
        },
    };

    let classes = state.classes();
    let sender = event.channel.take().expect("Failed to take sender");

    let request_object = classes
        .get(HttpRequestResource::CLASS_NAME)
        .expect("HttpRequestResource class not found")
        .object::<HttpRequest>(&ctx, Some(Box::new(HttpRequest::new(event.req))));
    let sender_object = classes
        .get(RequestEventResource::CLASS_NAME)
        .expect("RequestEventResource class not found")
        .object::<HttpResponseChannel>(&ctx, Some(Box::new(sender)));

    let object = JSObject::new(&ctx);
    object.set_property("request", &request_object, Default::default())?;
    object.set_property("sender", &sender_object, Default::default())?;
    Ok(object.into())
}

#[callback]
fn op_read_async_request_event(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    reader: JSObject,
    callback: JSObject,
) -> JSResult<JSValue> {
    let state = downcast_state(&ctx);
    let mut reader =
        match downcast_ref::<UnboundedBufferChannelReader<HttpRequestEvent>>(&reader) {
            Some(reader) => reader,
            None => {
                return Err(js_error_typ!(
                    &ctx,
                    "[Op:ReadRequestEvent] Invalid resource object"
                ))
            }
        };

    callback.protect();

    enqueue_job!(state, async move {
        let event = reader.read().await;

        native_job!("op_read_async_request_event", move |ctx| {
            let state = downcast_state(&ctx);
            let classes = state.classes();
            let mut event = match event {
                Ok(event) => event,
                Err(err) => match err {
                    StreamError::Closed | StreamError::Empty => {
                        let _ = callback.call(
                            None,
                            &[js_undefined!(&ctx), JSValue::number(&ctx, err.into())],
                        )?;
                        callback.unprotect();
                        return Ok(());
                    }
                    _ => {
                        let _ = callback
                            .call(None, &[js_error!(ctx, format!("{}", err)).into()])?;
                        callback.unprotect();
                        return Ok(());
                    }
                },
            };

            let sender = event.channel.take().expect("Failed to take sender");
            let request_object = classes
                .get(HttpRequestResource::CLASS_NAME)
                .expect("HttpRequestResource class not found")
                .object::<HttpRequest>(&ctx, Some(Box::new(HttpRequest::new(event.req))));
            let sender_object = classes
                .get(RequestEventResource::CLASS_NAME)
                .expect("RequestEventResource class not found")
                .object::<HttpResponseChannel>(&ctx, Some(Box::new(sender)));

            let object = JSObject::new(&ctx);
            object.set_property("request", &request_object, Default::default())?;
            object.set_property("sender", &sender_object, Default::default())?;
            let _ = callback.call(None, &[js_undefined!(&ctx), object.into()])?;
            callback.unprotect();
            Ok(())
        })
    });

    Ok(js_undefined!(&ctx))
}

async fn handle_server_shutdown(
    server: HttpServer,
    mut signal: Option<OneshotSignal>,
) -> NativeJob {
    let shutdown = server.listen();

    tokio::select! {
        // Handle Ctrl-C signal
        _ = tokio::signal::ctrl_c() => {
            println!("\nReceived Ctrl-C, shutting down server...");
            shutdown.shutdown();
        }
        _ = poll_fn(move |cx| {
            if let Some(sig) = signal.as_mut() {
                return sig.poll_signal(cx)
            } else {
                // Never resolves if no signal is provided
                std::task::Poll::Pending
            }
        }) => {
            println!("Received shutdown signal, shutting down server...");
            shutdown.shutdown();
        }
    }

    NativeJob::new(|_| Ok(()))
}

#[callback]
fn op_internal_start_server(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    options_args: JSValue,
    callback: JSObject,
) -> JSResult<JSValue> {
    callback.protect();
    let options = ServerOptions::from_value(&options_args, &ctx)?;
    let signal = options_args.as_object()?.get_property("signal")?;
    let mut internal_signal = None;
    if !signal.is_null() && signal.is_object() {
        let signal = signal.as_object()?;
        signal.protect();

        internal_signal = match downcast_ref::<InternalSignal>(&signal) {
            Some(mut internal) => internal.get_signal(),
            None => return Err(js_error!(&ctx, "Invalid Signal Object").into()),
        };
    }

    let server = HttpServerBuilder::new(HttpSocketAddr::IpSocket(options.address()))
        .config(HttpConfig::default())
        .bind();

    enqueue_job!(downcast_state(&ctx), async move {
        let server = server.await;

        native_job!("op_internal_start_server", move |ctx| {
            let state = downcast_state(&ctx);

            match server {
                Ok((http_server, reader)) => {
                    let reader_object = state
                        .classes()
                        .get(NetworkBufferChannelReaderResource::CLASS_NAME)
                        .expect("NetworkBufferChannelReaderResource not found")
                        .object(&ctx, Some(Box::new(reader)));

                    let shutdown_signal =
                        handle_server_shutdown(http_server, internal_signal);
                    state.job_queue().borrow().spawn(Box::pin(shutdown_signal));

                    let address = JSValue::string(&ctx, format!("{}", options.address()));
                    let object = JSObject::new(&ctx);
                    object.set_property("address", &address, Default::default())?;
                    object.set_property("reader", &reader_object, Default::default())?;
                    callback.call(None, &[js_undefined!(&ctx), object.into()])?;
                }
                Err(err) => {
                    let error = js_error!(ctx, format!("{}", err));
                    callback.call(None, &[error.into()])?;
                }
            };

            callback.unprotect();
            Ok(())
        })
    });

    Ok(js_undefined!(&ctx))
}

pub fn server_exports(ctx: &JSContext, exports: &JSObject) {
    let op_internal_start_server_fn = JSFunction::callback(
        ctx,
        Some("op_internal_start_server"),
        Some(op_internal_start_server),
    );

    let op_send_event_response_fn = JSFunction::callback(
        ctx,
        Some("op_send_event_response"),
        Some(op_send_event_response),
    );

    exports
        .set_property(
            "op_internal_start_server",
            &op_internal_start_server_fn,
            Default::default(),
        )
        .expect("Unable to set server property");

    exports
        .set_property(
            "op_send_event_response",
            &op_send_event_response_fn,
            Default::default(),
        )
        .expect("Unable to set server property");

    let op_read_request_event_fn = JSFunction::callback(
        ctx,
        Some("op_read_request_event"),
        Some(op_read_request_event),
    );
    exports
        .set_property(
            "op_read_request_event",
            &op_read_request_event_fn,
            Default::default(),
        )
        .expect("Unable to set server property");
    let op_read_async_request_event_fn = JSFunction::callback(
        ctx,
        Some("op_read_async_request_event"),
        Some(op_read_async_request_event),
    );
    exports
        .set_property(
            "op_read_async_request_event",
            &op_read_async_request_event_fn,
            Default::default(),
        )
        .expect("Unable to set server property");
}

/// | ---------------------- Event Request ---------------------- |
///
/// This class is used to create a JS object that wraps a decoded stream.
/// The object is used to read the decoded stream in chunks.
pub struct RequestEventResource {}

impl RequestEventResource {
    pub const CLASS_NAME: &'static str = "RequestEventResource";

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
    fn finalize(_: PrivateData) {
        // Any necessary cleanup can be implemented here
    }

    #[constructor]
    fn constructor(
        ctx: JSContext,
        constructor: JSObject,
        _args: &[JSValue],
    ) -> JSResult<JSValue> {
        let state = downcast_state(&ctx);
        let class = match state.classes().get(RequestEventResource::CLASS_NAME) {
            Some(class) => class,
            None => Err(JSError::new_typ(&ctx, "RequestEvent class not found")?)?,
        };

        let object = class.object::<HttpResponseChannel>(&ctx, None);
        object.set_prototype(&constructor);
        Ok(object.into())
    }
}

#[callback]
fn op_send_event_response(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    sender: JSObject,
    response: JSObject,
) -> JSResult<JSValue> {
    let fetch_response = HttpResponse::from_object(&ctx, &response)?;
    let http_response = match fetch_response.try_into() {
        Ok(response) => response,
        Err(err) => return Err(JSError::new_typ(&ctx, format!("{}", err))?)?,
    };

    let http_sender = match downcast_ref::<HttpResponseChannel>(&sender) {
        Some(sender) => sender.take(),
        None => {
            return Err(js_error_typ!(
                &ctx,
                "[Op:SendResponse] Invalid resource object"
            ))
        }
    };

    let _ = http_sender.send(http_response);
    Ok(JSValue::undefined(&ctx))
}
