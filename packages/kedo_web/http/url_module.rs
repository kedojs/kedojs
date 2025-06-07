use super::url_record::{parse_url, UrlRecord};
use crate::encoding::text_decoder_inner::EncodingTextDecoder;
use kedo_core::define_exports;
use rust_jsc::{callback, JSArray, JSContext, JSError, JSObject, JSResult, JSValue};

#[callback]
pub fn parse_url_encoded_form(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    if args.len() == 0 {
        return Err(JSError::new_typ(&ctx, "Expected 1 argument")?);
    }

    let input = args[0].as_string()?;
    let pairs: Vec<(String, String)> =
        form_urlencoded::parse(input.to_string().as_bytes())
            .into_iter()
            .map(|(k, v)| (k.as_ref().to_owned(), v.as_ref().to_owned()))
            .collect();
    let result = JSArray::new_array(&ctx, &[])?;

    for (_i, (k, v)) in pairs.iter().enumerate() {
        let pair = JSArray::new_array(
            &ctx,
            &[
                JSValue::string(&ctx, k.to_owned()),
                JSValue::string(&ctx, v.to_owned()),
            ],
        )?;
        result.push(&pair)?;
    }

    Ok(result.into())
}

#[callback]
pub fn serialize_url_encoded_form(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    if args.len() == 0 || !args[0].is_array() {
        return Err(JSError::new_typ(&ctx, "Expected 1 argument")?);
    }

    let list = JSArray::new(args[0].as_object()?);
    let len = list.length()? as u32;

    for i in 0..len {
        let pair = JSArray::new(list.get(i)?.as_object()?);
        let key = pair.get(0)?.as_string()?;
        let value = pair.get(1)?.as_string()?;
        pairs.push((key.to_string(), value.to_string()));
    }

    let result = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish();

    Ok(JSValue::string(&ctx, result))
}

#[callback]
fn basic_url_parse(
    ctx: JSContext,
    _: JSObject,
    _this: JSObject,
    args: &[JSValue],
) -> JSResult<JSValue> {
    return match parse_url(&ctx, args) {
        Ok(url) => {
            let result = JSObject::new(&ctx);
            result.set_property(
                "scheme",
                &JSValue::string(&ctx, url.scheme()),
                Default::default(),
            )?;
            result.set_property(
                "username",
                &JSValue::string(&ctx, url.username()),
                Default::default(),
            )?;
            result.set_property(
                "password",
                &JSValue::string(&ctx, url.password().unwrap_or("")),
                Default::default(),
            )?;
            result.set_property(
                "host",
                &JSValue::string(&ctx, url.host_str().unwrap_or("")),
                Default::default(),
            )?;
            result.set_property(
                "port",
                &JSValue::number(&ctx, url.port().unwrap_or(0) as f64),
                Default::default(),
            )?;
            result.set_property(
                "path",
                &JSValue::string(&ctx, url.path()),
                Default::default(),
            )?;
            result.set_property(
                "query",
                &JSValue::string(&ctx, url.query().unwrap_or("")),
                Default::default(),
            )?;
            result.set_property(
                "fragment",
                &JSValue::string(&ctx, url.fragment().unwrap_or("")),
                Default::default(),
            )?;
            Ok(result.into())
        }
        Err(e) => Err(JSError::new_typ(&ctx, e.to_string())?),
    };
}

pub struct UrlModule {}

define_exports!(
    UrlModule,
    @template[EncodingTextDecoder, UrlRecord],
    @function[parse_url_encoded_form, serialize_url_encoded_form, basic_url_parse]
);

// #[cfg(test)]
// mod tests {
//     use crate::tests::test_utils::new_runtime;

//     use super::*;

//     #[test]
//     fn test_parse_url_encoded_form() {
//         let ctx = new_runtime();
//         let result = ctx.evaluate_module_from_source(
//             r#"
//             import { parse_url_encoded_form } from '@kedo:op/web';
//             globalThis.form = parse_url_encoded_form('%24foo=%24bar&baz=%25qux');
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_ok());
//         let result = ctx.evaluate_script("globalThis.form", None);
//         assert!(result.is_ok());
//         let result = result.unwrap();
//         assert!(result.is_object());
//         let obj = result.as_object().unwrap();
//         let array = JSArray::new(obj);
//         assert_eq!(array.length().unwrap(), 2 as f64);
//         let foo = array.get(0).unwrap().as_object().unwrap();
//         let foo = JSArray::new(foo);
//         let baz = array.get(1).unwrap().as_object().unwrap();
//         let baz = JSArray::new(baz);
//         assert_eq!(foo.get(0).unwrap().as_string().unwrap(), "$foo");
//         assert_eq!(baz.get(0).unwrap().as_string().unwrap(), "baz");
//         assert_eq!(foo.get(1).unwrap().as_string().unwrap(), "$bar");
//         assert_eq!(baz.get(1).unwrap().as_string().unwrap(), "%qux");
//     }

//     #[test]
//     fn test_serialize_url_encoded_form() {
//         let rt = new_runtime();
//         let result = rt.evaluate_module_from_source(
//             r#"
//             import { serialize_url_encoded_form } from '@kedo:op/web';
//             globalThis.form = serialize_url_encoded_form([['$foo', '$bar'], ['baz', '%qux']]);
//             globalThis.form2 = serialize_url_encoded_form([['$foo', 'webdev'], ['baz', 'More webdev']]);
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_ok());
//         let result = rt.evaluate_script("globalThis.form", None);
//         assert!(result.is_ok());
//         let result = result.unwrap();
//         let result = result.as_string().unwrap();
//         assert_eq!(result, "%24foo=%24bar&baz=%25qux");

//         let result = rt.evaluate_script("globalThis.form2", None);
//         assert!(result.is_ok());
//         let result = result.unwrap();
//         let result = result.as_string().unwrap();
//         assert_eq!(result, "%24foo=webdev&baz=More+webdev");
//     }

//     #[test]
//     fn test_basic_url_parse() {
//         let ctx = new_runtime();
//         let result = ctx.evaluate_module_from_source(
//             r#"
//             import { basic_url_parse } from '@kedo:op/web';
//             globalThis.url = basic_url_parse('https://example.com:8080/foo/bar?baz=qux#quux');
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_ok());
//         let result = ctx.evaluate_script("globalThis.url", None);
//         assert!(result.is_ok());
//         let result = result.unwrap();
//         assert!(result.is_object());
//         let obj = result.as_object().unwrap();
//         assert_eq!(
//             obj.get_property("scheme").unwrap().as_string().unwrap(),
//             "https"
//         );
//         assert_eq!(
//             obj.get_property("host").unwrap().as_string().unwrap(),
//             "example.com"
//         );
//         assert_eq!(
//             obj.get_property("port").unwrap().as_number().unwrap(),
//             8080 as f64
//         );
//         assert_eq!(
//             obj.get_property("path").unwrap().as_string().unwrap(),
//             "/foo/bar"
//         );
//         assert_eq!(
//             obj.get_property("query").unwrap().as_string().unwrap(),
//             "baz=qux"
//         );
//         assert_eq!(
//             obj.get_property("fragment").unwrap().as_string().unwrap(),
//             "quux"
//         );
//     }

//     #[test]
//     fn test_basic_url_parse_with_base_url() {
//         let ctx = new_runtime();
//         let result = ctx.evaluate_module_from_source(
//             r#"
//             import { basic_url_parse } from '@kedo:op/web';
//             globalThis.url = basic_url_parse('foo/bar?baz=qux#quux', 'https://example.com:8080');
//         "#,
//             "index.js",
//             None,
//         );

//         assert!(result.is_ok());
//         let result = ctx.evaluate_script("globalThis.url", None);
//         assert!(result.is_ok());
//         let result = result.unwrap();
//         assert!(result.is_object());
//         let obj = result.as_object().unwrap();
//         assert_eq!(
//             obj.get_property("scheme").unwrap().as_string().unwrap(),
//             "https"
//         );
//         assert_eq!(
//             obj.get_property("host").unwrap().as_string().unwrap(),
//             "example.com"
//         );
//         assert_eq!(
//             obj.get_property("port").unwrap().as_number().unwrap(),
//             8080 as f64
//         );
//         assert_eq!(
//             obj.get_property("path").unwrap().as_string().unwrap(),
//             "/foo/bar"
//         );
//         assert_eq!(
//             obj.get_property("query").unwrap().as_string().unwrap(),
//             "baz=qux"
//         );
//         assert_eq!(
//             obj.get_property("fragment").unwrap().as_string().unwrap(),
//             "quux"
//         );
//     }
// }
