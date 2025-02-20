use kedo_std::HeadersMap;
use rust_jsc::{JSArray, JSObject, JSResult, JSString};

pub trait HeadersMapExt {
    fn into_value(
        &self,
        ctx: &rust_jsc::JSContext,
    ) -> rust_jsc::JSResult<rust_jsc::JSValue>;
    fn from_array(value: JSArray) -> JSResult<HeadersMap>;
}

impl HeadersMapExt for HeadersMap {
    fn into_value(
        &self,
        ctx: &rust_jsc::JSContext,
    ) -> rust_jsc::JSResult<rust_jsc::JSValue> {
        let mut response_headers: Vec<rust_jsc::JSValue> = vec![];
        for (key, value) in self.inner.iter() {
            let key = rust_jsc::JSValue::string(ctx, JSString::from(key.as_str()));
            let value = rust_jsc::JSValue::string(ctx, JSString::from(value.as_str()));
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

    fn from_array(value: JSArray) -> JSResult<HeadersMap> {
        let length = value.length()? as u32;
        let mut headers = Vec::new();

        for i in 0..length {
            let entry: JSObject = value.get(i)?.as_object()?;
            let key = entry.get_property_at_index(0)?.as_string()?;
            let value = entry.get_property_at_index(1)?.as_string()?;
            headers.push((key.to_string(), value.to_string()));
        }

        Ok(HeadersMap::new(headers))
    }
}

//     fn headers_map_into_value(
//         &self,
//         ctx: &rust_jsc::JSContext,
//     ) -> rust_jsc::JSResult<rust_jsc::JSValue>;
//     fn headers_map_from_array(value: JSArray) -> JSResult<HeadersMap>;
// }

// impl TryFromValue for HeadersMap {
//     fn from_value(value: JSValue) -> rust_jsc::JSResult<rust_jsc::JSValue> {
//         todo!()
//     }
// }
// fn from_value(value: JSValue) -> rust_jsc::JSResult<rust_jsc::JSValue> {
//     todo!()
// }

// Removed explicit impl for HeadersMapExt for HeadersMap to avoid conflicting implementations.

// pub fn headers_map_into_value(
//     headers_map: &HeadersMap,
//     ctx: &rust_jsc::JSContext,
// ) -> rust_jsc::JSResult<rust_jsc::JSValue> {
//     let mut response_headers: Vec<rust_jsc::JSValue> = vec![];
//     for (key, value) in headers_map.inner.iter() {
//         let key = rust_jsc::JSValue::string(ctx, JSString::from(key.as_str()));
//         let value = rust_jsc::JSValue::string(ctx, JSString::from(value.as_str()));
//         let header = JSArray::new_array(ctx, &[key, value])?;
//         // We need to protect the header object to prevent it from being garbage collected
//         header.protect();
//         response_headers.push(header.into());
//     }

//     let headers = JSArray::new_array(ctx, response_headers.as_slice())?;
//     // Then unprotect the headers array to prevent memory leaks
//     response_headers
//         .iter()
//         .for_each(|header| header.unprotect());
//     Ok(headers.into())
// }

// pub fn headers_map_from_array(value: JSArray) -> JSResult<HeadersMap> {
//     let length = value.length()? as u32;
//     let mut headers = Vec::new();

//     for i in 0..length {
//         let entry: JSObject = value.get(i)?.as_object()?;
//         let key = entry.get_property_at_index(0)?.as_string()?;
//         let value = entry.get_property_at_index(1)?.as_string()?;
//         headers.push((key.to_string(), value.to_string()));
//     }

//     Ok(HeadersMap { inner: headers })
// }

// #[cfg(test)]
// mod tests {
//     use rust_jsc::{JSContext, JSValue};

//     use super::*;
// }
