use rust_jsc::{JSArray, JSObject, JSString};

#[derive(Debug, Clone, Default)]
pub struct HeadersMap {
    pub inner: Vec<(String, String)>,
}

impl HeadersMap {
    pub fn new(inner: Vec<(String, String)>) -> Self {
        Self { inner }
    }

    // return the inner vector
    pub fn into_inner(&self) -> &Vec<(String, String)> {
        &self.inner
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.inner
            .iter()
            .find(|(k, _)| k.to_lowercase() == key.to_lowercase())
            .map(|(_, v)| v)
    }

    pub fn get_all(&self, key: &str) -> Vec<&String> {
        self.inner
            .iter()
            .filter(|(k, _)| k.to_lowercase() == key.to_lowercase())
            .map(|(_, v)| v)
            .collect()
    }

    pub fn set(&mut self, key: &str, value: &str) {
        let index = self
            .inner
            .iter()
            .position(|(k, _)| k.to_lowercase() == key.to_lowercase());
        match index {
            Some(i) => self.inner[i] = (key.to_string(), value.to_string()),
            None => self.inner.push((key.to_string(), value.to_string())),
        }
    }

    pub fn append(&mut self, key: &str, value: &str) {
        self.inner.push((key.to_string(), value.to_string()));
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        let index = self
            .inner
            .iter()
            .position(|(k, _)| k.to_lowercase() == key.to_lowercase());
        match index {
            Some(i) => Some(self.inner.remove(i).1),
            None => None,
        }
    }

    pub fn to_value(
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
}

impl TryFrom<JSArray> for HeadersMap {
    type Error = rust_jsc::JSError;

    fn try_from(value: JSArray) -> Result<Self, Self::Error> {
        let length = value.length()? as u32;
        let mut headers = Vec::new();

        for i in 0..length {
            let entry: JSObject = value.get(i)?.as_object()?;
            let key = entry.get_property_at_index(0)?.as_string()?;
            let value = entry.get_property_at_index(1)?.as_string()?;
            headers.push((key.to_string(), value.to_string()));
        }

        Ok(HeadersMap { inner: headers })
    }
}

// iterable headers
impl IntoIterator for HeadersMap {
    type Item = (String, String);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use rust_jsc::{JSContext, JSValue};

    use super::*;

    #[test]
    fn test_headers_map_new() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];
        let headers_map = HeadersMap::new(headers.clone());
        assert_eq!(headers_map.inner, headers);
    }

    #[test]
    fn test_headers_map_from_jsarray() {
        let ctx = JSContext::new();

        let entry1 = JSArray::new_array(
            &ctx,
            &vec![
                JSValue::string(&ctx, "Content-Type"),
                JSValue::string(&ctx, "application/json"),
            ],
        )
        .unwrap();
        let entry2 = JSArray::new_array(
            &ctx,
            &vec![
                JSValue::string(&ctx, "Accept"),
                JSValue::string(&ctx, "application/json"),
            ],
        )
        .unwrap();

        let array =
            JSArray::new_array(&ctx, &vec![entry1.into(), entry2.into()]).unwrap();

        let headers_map: HeadersMap = HeadersMap::try_from(array).unwrap();
        let expected_headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];
        assert_eq!(headers_map.inner, expected_headers);
    }

    #[test]
    fn test_headers_map_into_iter() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];
        let headers_map = HeadersMap::new(headers.clone());
        let iter = headers_map.into_iter();
        let collected: Vec<(String, String)> = iter.collect();
        assert_eq!(collected, headers);
    }

    #[test]
    fn test_headers_map_remove() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];
        let mut headers_map = HeadersMap::new(headers.clone());
        let removed = headers_map.remove("Content-Type");
        assert_eq!(removed, Some("application/json".to_string()));
        assert_eq!(
            headers_map.inner,
            vec![("Accept".to_string(), "application/json".to_string())]
        );
    }

    #[test]
    fn test_headers_map_set() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];
        let mut headers_map = HeadersMap::new(headers.clone());
        headers_map.set("Content-Type", "text/plain");
        assert_eq!(
            headers_map.inner,
            vec![
                ("Content-Type".to_string(), "text/plain".to_string()),
                ("Accept".to_string(), "application/json".to_string())
            ]
        );
    }

    #[test]
    fn test_headers_map_append() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];
        let mut headers_map = HeadersMap::new(headers.clone());
        headers_map.append("Authorization", "Bearer token");
        assert_eq!(
            headers_map.inner,
            vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Accept".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), "Bearer token".to_string())
            ]
        );
    }

    #[test]
    fn test_headers_map_get_all() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "text/plain".to_string()),
        ];
        let headers_map = HeadersMap::new(headers.clone());
        let all_accept = headers_map.get_all("Accept");
        assert_eq!(all_accept, vec!["application/json", "text/plain"]);
    }
}
