use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct HeadersMap {
    pub inner: Vec<(String, String)>,
    // a lookup cache mapping lower-case header names to their positions in `inner`
    lookup: HashMap<String, Vec<usize>>,
}

impl HeadersMap {
    pub fn new(inner: Vec<(String, String)>) -> Self {
        let mut headers_map = HeadersMap {
            inner,
            lookup: HashMap::new(),
        };
        for (i, (key, _)) in headers_map.inner.iter().enumerate() {
            let key_lc = key.to_lowercase();
            headers_map.lookup.entry(key_lc).or_default().push(i);
        }
        headers_map
    }

    // return the inner vector
    pub fn into_inner(&self) -> &Vec<(String, String)> {
        &self.inner
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.lookup
            .get(&key.to_lowercase())
            .and_then(|indices| indices.first().map(|&i| &self.inner[i].1))
    }

    pub fn get_all(&self, key: &str) -> Vec<&String> {
        if let Some(indices) = self.lookup.get(&key.to_lowercase()) {
            indices.iter().map(|&i| &self.inner[i].1).collect()
        } else {
            vec![]
        }
    }

    pub fn set(&mut self, key: &str, value: &str) {
        let key_lc = key.to_lowercase();
        if let Some(indices) = self.lookup.get_mut(&key_lc) {
            if !indices.is_empty() {
                let i = indices[0];
                // update the header at index `i`
                self.inner[i] = (key.to_string(), value.to_string());
                return;
            }
        }
        // if the key doesn't exist, push to the vector and update lookup
        self.inner.push((key.to_string(), value.to_string()));
        let idx = self.inner.len() - 1;
        self.lookup.entry(key_lc).or_default().push(idx);
    }

    pub fn append(&mut self, key: &str, value: &str) {
        self.inner.push((key.to_string(), value.to_string()));
        let idx = self.inner.len() - 1;
        self.lookup.entry(key.to_lowercase()).or_default().push(idx);
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        let key_lc = key.to_lowercase();
        if let Some(indices) = self.lookup.get_mut(&key_lc) {
            if indices.is_empty() {
                return None;
            }
            // remove the first occurrence
            let remove_index = indices.remove(0);
            let removed_value = self.inner.remove(remove_index).1;

            // Update indices in the lookup map that are greater than remove_index
            for indices in self.lookup.values_mut() {
                for idx in indices.iter_mut() {
                    if *idx > remove_index {
                        *idx -= 1;
                    }
                }
            }
            if let Some(v) = self.lookup.get(&key_lc) {
                if v.is_empty() {
                    self.lookup.remove(&key_lc);
                }
            }
            return Some(removed_value);
        }
        None
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
