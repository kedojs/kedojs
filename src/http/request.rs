use boa_engine::{Context, JsNativeError, JsValue};
use hyper::Uri;
use serde_json::Value;

pub struct WebRequest {
  pub method: String,
  pub uri: Uri,
  pub headers: Vec<(String, String)>,
  pub body: Option<Value>,
}

impl WebRequest {
  pub fn from_value(
    value: JsValue,
    uri: Uri,
    context: &mut Context,
  ) -> Result<Self, JsNativeError> {
    let binding = value.to_json(context).unwrap();

    if !binding.is_object() {
      return Err(JsNativeError::typ().with_message("Invalid request object"));
    }

    let object = binding.as_object().unwrap();
    let method = match object.get("method") {
      Some(method) => method.as_str().unwrap_or("GET").to_string(),
      None => "GET".to_string(),
    };

    let body = object
      .get("body")
      .unwrap_or_else(|| &serde_json::Value::Null);
    let body = match body {
      serde_json::Value::Null => None,
      _ => Some(body.clone()),
    };

    let default_headers = &serde_json::Value::Object(serde_json::Map::new());
    let headers = object.get("headers").unwrap_or(default_headers);
    let headers = match headers.as_object() {
      Some(headers) => headers.clone(),
      None => serde_json::Map::new(),
    };
    let mut headers_vec = Vec::new();
    for (key, value) in headers.iter() {
      headers_vec.push((key.to_string(), value.to_string()));
    }

    Ok(WebRequest {
      method,
      uri,
      headers: headers_vec,
      body,
    })
  }
}
