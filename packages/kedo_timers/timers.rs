use kedo_core::{downcast_state, JsProctectedCallable};
use kedo_utils::define_globals;
use rust_jsc::{callback, JSContext, JSFunction, JSObject, JSResult, JSValue};
use tokio::time::Duration;

pub struct Timer {}

impl Timer {
    #[callback]
    fn set_timeout(
        ctx: JSContext,
        _: JSObject,
        __: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let state = downcast_state(&ctx);
        let function = args[0].as_object()?;
        let function = JSFunction::from(function);
        let timeout = args[1].as_number()? as u64;
        let args = args[2..].to_vec();
        function.protect();
        let id = state.timers().add_timer(
            Duration::from_millis(timeout),
            kedo_std::TimerType::Timeout,
            JsProctectedCallable::new(function, args),
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
        let state = downcast_state(&ctx);
        let function = args[0].as_object()?;
        let function = JSFunction::from(function);
        let timeout = args[1].as_number()? as u64;
        let args = args[2..].to_vec();
        function.protect();
        let id = state.timers().add_timer(
            Duration::from_millis(timeout),
            kedo_std::TimerType::Interval,
            JsProctectedCallable::new(function, args),
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
        let state = downcast_state(&ctx);
        let timer_id = args[0].as_number()? as usize;
        let _ = state.timers().clear_timer(&timer_id);
        Ok(JSValue::undefined(&ctx))
    }
}

define_globals!(
    Timer,
    setTimeout => Timer::set_timeout,
    setInterval => Timer::set_interval,
    clearTimeout => Timer::clear_timer,
    clearInterval => Timer::clear_timer
);
