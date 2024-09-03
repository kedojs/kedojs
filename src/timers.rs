use rust_jsc::{callback, JSContext, JSFunction, JSObject, JSResult, JSValue};
use tokio::time::Duration;

use crate::context::KedoContext;

use super::timer_queue::{TimerJsCallable, TimerType};

pub struct Timer {}

impl Timer {
    const SET_TIMEOUT_NAME: &'static str = "setTimeout";
    const SET_INTERVAL_NAME: &'static str = "setInterval";
    const CLEAR_TIMEOUT_NAME: &'static str = "clearTimeout";
    const CLEAR_INTERVAL_NAME: &'static str = "clearInterval";

    pub fn init(ctx: &JSContext) -> JSResult<()> {
        let global_object = ctx.global_object();

        let timeout_callback = JSFunction::callback(
            ctx,
            Some(Self::SET_TIMEOUT_NAME),
            Some(Self::set_timeout),
        );
        global_object.set_property(
            Self::SET_TIMEOUT_NAME,
            &timeout_callback,
            Default::default(),
        )?;

        let interval_callback = JSFunction::callback(
            ctx,
            Some(Self::SET_INTERVAL_NAME),
            Some(Self::set_interval),
        );
        global_object.set_property(
            Self::SET_INTERVAL_NAME,
            &interval_callback,
            Default::default(),
        )?;

        let clear_timeout_callback = JSFunction::callback(
            ctx,
            Some(Self::CLEAR_TIMEOUT_NAME),
            Some(Self::clear_timer),
        );
        global_object.set_property(
            Self::CLEAR_TIMEOUT_NAME,
            &clear_timeout_callback,
            Default::default(),
        )?;

        global_object.set_property(
            Self::CLEAR_INTERVAL_NAME,
            &clear_timeout_callback,
            Default::default(),
        )?;

        Ok(())
    }

    #[callback]
    fn set_timeout(
        ctx: JSContext,
        _: JSObject,
        __: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let state = KedoContext::from(&ctx).state();
        let function = args[0].as_object()?;
        let function = JSFunction::from(function);
        let timeout = args[1].as_number()? as u64;
        let args = args[2..].to_vec();
        function.protect();
        let id = state.timers().add_timer(
            Duration::from_millis(timeout),
            TimerType::Timeout,
            TimerJsCallable {
                callable: function,
                args,
            },
            None,
        );
        Ok(JSValue::number(&ctx, id as f64))
    }

    #[callback]
    fn set_interval(
        ctx: JSContext,
        _: JSObject,
        __: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let state = KedoContext::from(&ctx).state();
        let function = args[0].as_object()?;
        let function = JSFunction::from(function);
        let timeout = args[1].as_number()? as u64;
        let args = args[2..].to_vec();
        function.protect();
        let id = state.timers().add_timer(
            Duration::from_millis(timeout),
            TimerType::Interval,
            TimerJsCallable {
                callable: function,
                args,
            },
            None,
        );
        Ok(JSValue::number(&ctx, id as f64))
    }

    #[callback]
    fn clear_timer(
        ctx: JSContext,
        _: JSObject,
        __: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let state = KedoContext::from(&ctx).state();
        let timer_id = args[0].as_number()? as usize;
        state.timers().clear_timer(&timer_id);
        Ok(JSValue::undefined(&ctx))
    }
}
