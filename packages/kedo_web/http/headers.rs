use rust_jsc::{JSArray, JSObject, JSResult, JSString};
use std::str::FromStr as _;

pub trait HeadersMapExt<T> {
    fn into_value(
        &self,
        ctx: &rust_jsc::JSContext,
    ) -> rust_jsc::JSResult<rust_jsc::JSValue>;
    fn from_array(value: JSArray) -> JSResult<T>;
}

impl HeadersMapExt<hyper::header::HeaderMap> for hyper::header::HeaderMap {
    fn into_value(
        &self,
        ctx: &rust_jsc::JSContext,
    ) -> rust_jsc::JSResult<rust_jsc::JSValue> {
        let mut response_headers: Vec<rust_jsc::JSValue> = vec![];
        for (key, value) in self.iter() {
            let key = rust_jsc::JSValue::string(ctx, JSString::from(key.as_str()));
            let value =
                rust_jsc::JSValue::string(ctx, JSString::from(value.to_str().unwrap()));
            let header = JSArray::new_array(ctx, &[key, value])?;
            // We need to protect the header object to prevent it from being garbage collected
            header.protect();
            response_headers.push(header.into());
        }

        let headers = JSArray::new_array(ctx, response_headers.as_slice())?;
        // Then unprotect the headers array to prevent memory leaks
        response_headers
            .iter()
            .for_each(|header| header.unprotect());
        Ok(headers.into())
    }

    fn from_array(value: JSArray) -> JSResult<hyper::header::HeaderMap> {
        let length = value.length()? as u32;
        let mut headers = hyper::header::HeaderMap::new();

        for i in 0..length {
            let entry: JSObject = value.get(i)?.as_object()?;
            let key = entry.get_property_at_index(0)?.as_string()?;
            let value = entry.get_property_at_index(1)?.as_string()?;
            headers.insert(
                hyper::header::HeaderName::from_str(key.to_string().as_str()).unwrap(),
                hyper::header::HeaderValue::from_str(value.to_string().as_str()).unwrap(),
            );
        }

        Ok(headers)
    }
}
