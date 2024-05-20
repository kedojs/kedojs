use rust_jsc::{callback, JSContext, JSFunction, JSObject, JSResult, JSString, JSValue};

#[derive(Debug)]
enum LogMessage {
    Log,
    Info,
    Warn,
    Error,
}

pub fn format_args(_ctx: &JSContext, args: &[JSValue]) -> Result<String, JSValue> {
    let mut arg_index = 1;
    let target = match args.get(0) {
        Some(value) => value,
        None => return Ok("".to_string()),
    };

    let mut formatted = target
        .as_string()
        .expect("Failed to convert to string")
        .to_string();

    while let Some(percent_pos) = formatted.find('%') {
        if arg_index >= args.len() {
            break;
        }

        let specifier = formatted.chars().nth(percent_pos + 1);
        match specifier {
            Some('o') | Some('O') => {
                let arg = &args[arg_index];
                let json = arg
                    .as_json_string(0)
                    .unwrap_or_else(|_| JSString::from("[Object]"));
                formatted = formatted.replace(
                    &formatted[percent_pos..=percent_pos + 1],
                    json.to_string().as_str(),
                );
            }
            Some('d') | Some('i') => {
                let arg = &args[arg_index].as_number().unwrap_or(0.into());
                formatted = formatted.replace(
                    &formatted[percent_pos..=percent_pos + 1],
                    &format!("{:}", arg),
                );
            }
            Some('s') => {
                let arg = &args[arg_index]
                    .as_string()
                    .unwrap_or_else(|_| JSString::from("[Invalid string]"));
                formatted = formatted.replace(
                    &formatted[percent_pos..=percent_pos + 1],
                    arg.to_string().as_str(),
                );
            }
            Some('f') => {
                let arg = &args[arg_index].as_number().unwrap_or(0.into());
                formatted = formatted.replace(
                    &formatted[percent_pos..=percent_pos + 1],
                    &format!("{:}", arg),
                );
            }
            _ => {}
        }

        arg_index += 1;
    }

    // add remaining args
    for arg in args.iter().skip(arg_index) {
        match arg.get_type() {
            rust_jsc::JSValueType::Number => {
                formatted.push_str(format!(" {}", arg.as_number().unwrap()).as_str());
            }
            rust_jsc::JSValueType::String => {
                formatted.push_str(format!(" {}", arg.as_string().unwrap()).as_str());
            }
            rust_jsc::JSValueType::Boolean => {
                formatted.push_str(format!(" {}", arg.as_boolean()).as_str());
            }
            rust_jsc::JSValueType::Null => {
                formatted.push_str(" null");
            }
            rust_jsc::JSValueType::Undefined => {
                formatted.push_str(" undefined");
            }
            rust_jsc::JSValueType::Object => {
                formatted.push_str(
                    format!(
                        " {}",
                        arg.as_json_string(0)
                            .unwrap_or_else(|_arg| { JSString::from("[Object]") })
                    )
                    .as_str(),
                );
            }
            _ => {}
        }
    }

    Ok(formatted)
}

pub struct Console;

impl Console {
    pub fn init(ctx: &rust_jsc::JSContext) -> JSResult<()> {
        let console = JSObject::new(ctx);

        let log_callback = JSFunction::callback(ctx, Some("log"), Some(Self::log));
        console.set_property("log", &log_callback, Default::default())?;

        let error_callback = JSFunction::callback(ctx, Some("error"), Some(Self::error));
        console.set_property("error", &error_callback, Default::default())?;

        let info_callback = JSFunction::callback(ctx, Some("info"), Some(Self::info));
        console.set_property("info", &info_callback, Default::default())?;

        let warn_callback = JSFunction::callback(ctx, Some("warn"), Some(Self::warn));
        console.set_property("warn", &warn_callback, Default::default())?;

        ctx.global_object()
            .set_property("console", &console, Default::default())?;
        Ok(())
    }

    #[callback]
    fn log(
        ctx: JSContext,
        _function: JSObject,
        _this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        Console::logger(LogMessage::Log, &ctx, args).unwrap();
        Ok(JSValue::undefined(&ctx))
    }

    #[callback]
    fn error(
        ctx: JSContext,
        _: JSObject,
        __: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        Console::logger(LogMessage::Error, &ctx, args).unwrap();
        Ok(JSValue::undefined(&ctx))
    }

    #[callback]
    fn info(
        ctx: JSContext,
        _: JSObject,
        __: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        Console::logger(LogMessage::Info, &ctx, args).unwrap();
        Ok(JSValue::undefined(&ctx))
    }

    #[callback]
    fn warn(
        ctx: JSContext,
        _: JSObject,
        __: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        Console::logger(LogMessage::Warn, &ctx, args).unwrap();
        Ok(JSValue::undefined(&ctx))
    }

    fn logger(
        log_msg: LogMessage,
        ctx: &rust_jsc::JSContext,
        args: &[rust_jsc::JSValue],
    ) -> Result<(), rust_jsc::JSValue> {
        let msg = format_args(&ctx, args)?;
        match log_msg {
            LogMessage::Error => {
                eprintln!("{msg}");
            }
            LogMessage::Log | LogMessage::Info | LogMessage::Warn => {
                println!("{msg}");
            }
        }
        Ok(())
    }
}
