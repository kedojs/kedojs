use crate::{
    http::{request::FetchRequestExt, response::FetchResponseExt},
    signals::{InternalSignal, OneshotSignal},
    FetchRequestResource, HttpRequestResource,
};
use futures::Stream;
use kedo_core::{
    downcast_state, enqueue_job, native_job, AsyncJobQueueInner, ClassTable, NativeJob,
};
use kedo_macros::js_class;
use kedo_std::{
    FetchRequest, FetchResponse, HttpRequest, HttpServerBuilder, HttpSocketAddr,
    RequestEvent, RequestEventSender, RequestReceiver, ServerHandle, StreamError,
    UnboundedBufferChannel, UnboundedBufferChannelReader, UnboundedBufferChannelWriter,
};
use kedo_utils::{downcast_ref, js_error, js_error_typ, js_undefined};
use rust_jsc::{
    callback, class::ClassError, constructor, finalize, JSClass, JSClassAttribute,
    JSContext, JSError, JSFunction, JSObject, JSResult, JSValue, PrivateData,
};
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    future::Future,
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    rc::Weak,
    vec,
};

struct ServerOptions {
    port: u16,
    hostname: String,
    address: SocketAddr,
    key: Option<String>,
    cert: Option<String>,
    // handler: JSObject,
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
        // let handler = value.get_property("handler")?.as_object()?;
        // handler.protect();

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

struct HttpAccepterBuilder {
    handler: Option<ServerHandle>,
    receiver: Option<RequestReceiver>,
    signal: Option<OneshotSignal>,
    // queue: Option<Weak<RefCell<AsyncJobQueueInner>>>,
    stream: Option<UnboundedBufferChannelWriter<RequestEvent>>,
    // function: Option<JSObject>,
}

impl HttpAccepterBuilder {
    pub fn new() -> Self {
        Self {
            handler: None,
            receiver: None,
            signal: None,
            // queue: None,
            stream: None,
            // function: None,
        }
    }

    pub fn handler(mut self, handler: ServerHandle) -> Self {
        self.handler = Some(handler);
        self
    }

    pub fn receiver(mut self, receiver: RequestReceiver) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub fn signal(mut self, signal: OneshotSignal) -> Self {
        self.signal = Some(signal);
        self
    }

    // pub fn queue(mut self, queue: Weak<RefCell<AsyncJobQueueInner>>) -> Self {
    //     self.queue = Some(queue);
    //     self
    // }

    // pub fn function(mut self, function: JSObject) -> Self {
    //     self.function = Some(function);
    //     self
    // }

    pub fn stream(mut self, stream: UnboundedBufferChannelWriter<RequestEvent>) -> Self {
        self.stream = Some(stream);
        self
    }

    pub fn build(self) -> HttpAccepter {
        // let stream: UnboundedBufferChannel<RequestEvent> = UnboundedBufferChannel::new();
        // let writer = stream.writer().unwrap();

        HttpAccepter {
            handler: self.handler,
            receiver: Box::pin(self.receiver.expect("receiver is required")),
            signal: self.signal.expect("signal is required"),
            // queue: self.queue.expect("queue is required"),
            stream: self.stream.expect("stream is required"),
            // function: self.function.expect("function is required"),
        }
    }
}

struct HttpAccepter {
    handler: Option<ServerHandle>,
    receiver: Pin<Box<RequestReceiver>>,
    signal: OneshotSignal,
    // queue: Weak<RefCell<AsyncJobQueueInner>>,
    stream: UnboundedBufferChannelWriter<RequestEvent>,
    // function: JSObject,
}

// impl Drop for HttpAccepter {
//     fn drop(&mut self) {
//         self.function.unprotect();
//     }
// }

#[js_class(
    resource = UnboundedBufferChannelReader<RequestEvent>,
    proto = "UnboundedBufferChannelReaderPrototype",
)]
pub struct UnboundedBufferChannelReaderResource {}

impl HttpAccepter {
    fn shutdown(&mut self) {
        match self.handler.take() {
            Some(handler) => {
                handler.shutdown();
            }
            None => {}
        };
    }

    fn is_shutdown(&self) -> bool {
        self.handler.is_none()
    }

    // fn process_request(mut event: RequestEvent, function: JSObject) -> NativeJob {
    //     // TODO: close the connection on error
    //     let sender = event.sender.take().expect("Failed to take sender");
    //     let request = FetchRequest::from_event(event).expect("Failed to convert request");

    //     NativeJob::new(move |ctx| {
    //         let state = downcast_state(&ctx);
    //         let classes = state.classes();
    //         let request_object = classes
    //             .get(FetchRequestResource::CLASS_NAME)
    //             .expect("FetchRequestResource class not found")
    //             .object::<FetchRequest>(&ctx, Some(Box::new(request)));
    //         let sender = classes
    //             .get(RequestEventResource::CLASS_NAME)
    //             .expect("RequestEventResource class not found")
    //             .object::<RequestEventSender>(&ctx, Some(Box::new(sender)));

    //         let _ = function.call(None, &[request_object.into(), sender.into()])?;
    //         Ok(())
    //     })
    // }
}

impl Future for HttpAccepter {
    type Output = NativeJob;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let shutdown_signal = self.signal.poll_signal(cx);
        if let std::task::Poll::Ready(Ok(())) = shutdown_signal {
            match self.handler.take() {
                Some(handler) => {
                    println!("Shutting down the server");
                    handler.shutdown();
                }
                None => {}
            };

            return std::task::Poll::Ready(NativeJob::new(|_| Ok(())));
        };

        // let mut http_events = vec![];
        let mut is_channel_closed = false;
        loop {
            match self.receiver.as_mut().poll_next(cx) {
                std::task::Poll::Ready(Some(event)) => {
                    self.stream.try_write(event).expect("Failed to send event");
                    // http_events.push(event)
                }
                std::task::Poll::Pending => break,
                std::task::Poll::Ready(None) => {
                    is_channel_closed = true;
                    // The channel has been closed
                    break;
                }
            };
        }

        if is_channel_closed {
            return std::task::Poll::Ready(NativeJob::new(|_| Ok(())));
        }

        // if http_events.is_empty() {
        //     if is_channel_closed {
        //         return std::task::Poll::Ready(NativeJob::new(|_| Ok(())));
        //     }

        //     return std::task::Poll::Pending;
        // }

        // if let Some(queue_rc) = self.queue.upgrade() {
        //     let queue_refcell: &RefCell<AsyncJobQueueInner> =
        //         std::rc::Rc::as_ref(&queue_rc);

        //     let function = self.function.clone();
        //     queue_refcell
        //         .borrow_mut()
        //         .push_job(NativeJob::new(move |ctx| {
        //             while let Some(mut event) = http_events.pop() {
        //                 let sender = event.sender.take().expect("Failed to take sender");
        //                 let request = HttpRequest::new(event.req);
        //                 // FetchRequest::from_event(event)
        //                 //     .expect("Failed to convert request");

        //                 let state = downcast_state(&ctx);
        //                 let classes = state.classes();
        //                 let request_object = classes
        //                     .get(HttpRequestResource::CLASS_NAME)
        //                     .expect("HttpRequestResource class not found")
        //                     .object::<HttpRequest>(&ctx, Some(Box::new(request)));
        //                 let sender = classes
        //                     .get(RequestEventResource::CLASS_NAME)
        //                     .expect("RequestEventResource class not found")
        //                     .object::<RequestEventSender>(&ctx, Some(Box::new(sender)));

        //                 let _ = function
        //                     .call(None, &[request_object.into(), sender.into()])?;
        //             }

        //             Ok(())
        //         }));

        //     if is_channel_closed {
        //         return std::task::Poll::Ready(NativeJob::new(|_| Ok(())));
        //     }

        return std::task::Poll::Pending;
        // };

        // std::task::Poll::Ready(NativeJob::new(|_| Ok(())))
    }
}

#[callback]
fn op_read_request_event(
    ctx: JSContext,
    _: JSObject,
    _: JSObject,
    reader: JSObject,
) -> JSResult<JSValue> {
    let state = downcast_state(&ctx);
    let reader = downcast_ref::<UnboundedBufferChannelReader<RequestEvent>>(&reader);
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
            StreamError::Empty => {
                return Ok(js_undefined!(&ctx));
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
    let sender = event.sender.take().expect("Failed to take sender");
    // let request = FetchRequest::from_event(event).expect("Failed to convert request");

    let request_object = classes
        .get(HttpRequestResource::CLASS_NAME)
        .expect("FetchResponse class not found")
        .object::<HttpRequest>(&ctx, Some(Box::new(HttpRequest::new(event.req))));
    let sender_object = classes
        .get(RequestEventResource::CLASS_NAME)
        .expect("RequestEventResource class not found")
        .object::<RequestEventSender>(&ctx, Some(Box::new(sender)));

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
        match downcast_ref::<UnboundedBufferChannelReader<RequestEvent>>(&reader) {
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
                    StreamError::Empty => {
                        let _ = callback.call(None, &[js_undefined!(&ctx)])?;
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

            let sender = event.sender.take().expect("Failed to take sender");
            // let request =
            //     FetchRequest::from_event(event).expect("Failed to convert request");

            let request_object = classes
                .get(HttpRequestResource::CLASS_NAME)
                .expect("FetchResponse class not found")
                .object::<HttpRequest>(&ctx, Some(Box::new(HttpRequest::new(event.req))));
            let sender_object = classes
                .get(RequestEventResource::CLASS_NAME)
                .expect("RequestEventResource class not found")
                .object::<RequestEventSender>(&ctx, Some(Box::new(sender)));

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
    if !signal.is_undefined() && signal.is_object() {
        let signal = signal.as_object()?;
        signal.protect();
        let oneshot_signal = downcast_ref::<InternalSignal>(&signal)
            .map(|mut signal| signal.as_mut().get_signal());
        if let Some(signal) = oneshot_signal {
            internal_signal = signal;
        }
    }

    let server = HttpServerBuilder::new()
        .addr(HttpSocketAddr::IpSocket(options.address()))
        .start();

    let mut channel: UnboundedBufferChannel<RequestEvent> = UnboundedBufferChannel::new();
    let writer = channel.writer().unwrap();
    let reader = channel.aquire_reader().unwrap();

    let state = downcast_state(&ctx);
    let reader_object = state
        .classes()
        .get(UnboundedBufferChannelReaderResource::CLASS_NAME)
        .unwrap()
        .object(&ctx, Some(Box::new(reader)));

    enqueue_job!(state, async move {
        let server = server.await;
        // {
        //     Ok(server) => server,
        //     Err(err) => {
        //         // let err_value = JSError::with_message(&ctx, format!("{}", err)).unwrap();
        //         // resolver.reject(None, &[err_value.into()])?;
        //         // return Ok(());
        //     }
        // };
        // let (mut handler, mut receiver) = server.await.accept().unwrap();

        native_job!("op_internal_start_server", move |ctx| {
            let state = downcast_state(&ctx);
            let job_queue = state.job_queue().borrow().borrow_mut().leak();

            match server {
                Ok(http_server) => {
                    let accepter = http_server.accept();
                    match accepter {
                        Ok((handler, receiver)) => {
                            let address =
                                JSValue::string(&ctx, format!("{}", options.address()));
                            let accepter = HttpAccepterBuilder::new()
                                .handler(handler)
                                .receiver(receiver)
                                .signal(internal_signal.unwrap())
                                .stream(writer)
                                // .queue(job_queue)
                                // .function(options.handler)
                                .build();

                            state.job_queue().borrow().spawn(Box::pin(accepter));
                            // callback.call(None, &[js_undefined!(&ctx), address])?;
                            callback.call(
                                None,
                                &[js_undefined!(&ctx), reader_object.into()],
                            )?;
                        }
                        Err(err) => {
                            let error = js_error!(ctx, format!("{}", err));
                            callback.call(None, &[error.into()])?;
                        }
                    }
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

        let object = class.object::<RequestEventSender>(&ctx, None);
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
    let fetch_response = FetchResponse::from_object(&ctx, &response)?;
    let http_response = match fetch_response.try_into() {
        Ok(response) => response,
        Err(err) => return Err(JSError::new_typ(&ctx, format!("{}", err))?)?,
    };

    let http_sender = match downcast_ref::<RequestEventSender>(&sender) {
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

#[cfg(test)]
mod tests {
    use futures::{future::poll_fn, stream::FuturesUnordered, StreamExt};
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll, Waker},
    };
    use tokio::sync::mpsc::UnboundedReceiver;

    pub struct HttpAccepter {
        receiver: UnboundedReceiver<String>,
        count: usize,
        waker: Option<Waker>,
    }

    impl HttpAccepter {
        pub fn new(receiver: UnboundedReceiver<String>) -> Self {
            Self {
                receiver,
                count: 0,
                waker: None,
            }
        }

        pub fn wake(&mut self) {
            if let Some(waker) = self.waker.take() {
                waker.wake();
            }
        }
    }

    impl Future for HttpAccepter {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            self.waker = Some(cx.waker().clone());

            while let Poll::Ready(Some(event)) = self.receiver.poll_recv(cx) {
                self.count += 1;
                // Handle the event
                println!("Received event: {:?}", event);
            }

            if self.count > 3 {
                return Poll::Ready(());
            }

            Poll::Pending
        }
    }

    #[tokio::test]
    async fn test_http_accepter() {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let accepter = Box::pin(HttpAccepter::new(receiver));
        let mut queue = FuturesUnordered::new();

        // let waker = futures::task::noop_waker();
        // let cx = std::task::Context::from_waker(&waker);

        sender.send(String::from("Hello")).unwrap();

        queue.push(accepter);
        poll_fn(move |cx| {
            // let result = accepter.as_mut().poll(cx);
            // sender.send(String::from("Hello")).unwrap();
            // result
            while let Poll::Ready(Some(_)) = queue.poll_next_unpin(cx) {
                println!("The accepter has been polled");
            }

            sender.send(String::from("Hello")).unwrap();
            if queue.is_empty() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
    }
}
