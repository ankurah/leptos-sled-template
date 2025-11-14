use wasm_bindgen::{JsCast, JsValue};

/// Helper trait for reducing boilerplate when working with JsValue results.
/// Similar to `expect` but tailored for wasm-bindgen use cases.
pub trait Require<T> {
    fn require(self, msg: &str) -> Result<T, String>;
}

impl<T> Require<T> for Result<T, JsValue> {
    fn require(self, msg: &str) -> Result<T, String> {
        self.map_err(|e| {
            let error_msg = if let Some(s) = e.as_string() {
                s
            } else if let Some(obj) = e.dyn_ref::<js_sys::Object>() {
                js_sys::JSON::stringify(obj).ok().and_then(|v| v.as_string()).unwrap_or_else(|| "[object]".to_string())
            } else {
                format!("{:?}", e)
            };
            format!("{}: {}", msg, error_msg)
        })
    }
}

impl<T> Require<T> for Option<T> {
    fn require(self, msg: &str) -> Result<T, String> {
        self.ok_or_else(|| format!("{}: None", msg))
    }
}

impl<T> Require<T> for Result<Option<T>, JsValue> {
    fn require(self, msg: &str) -> Result<T, String> {
        match self {
            Ok(Some(v)) => Ok(v),
            Ok(None) => Err(format!("{}: None", msg)),
            Err(e) => Err(format!("{}: {:?}", msg, e)),
        }
    }
}
