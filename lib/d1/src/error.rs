use thiserror::Error;
use wasm_bindgen::JsValue;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DatabaseError {
    #[error("Binding cannot be cast")]
    BindingCast,

    #[error("Failed to await promise: {0}")]
    AwaitPromise(String),

    #[error("Failed to cast a JsValue")]
    JsCast,

    #[error("Failed to bind a parameter")]
    BindParameter,

    #[error("serde-wasm-bindgen error: {0}")]
    SerdeWasmBindgen(String),

    #[error(transparent)]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

impl From<serde_wasm_bindgen::Error> for DatabaseError {
    fn from(e: serde_wasm_bindgen::Error) -> Self {
        let val: JsValue = e.into();
        let msg = val.as_string().unwrap_or_else(|| "unknown".to_string());
        DatabaseError::SerdeWasmBindgen(msg)
    }
}

impl From<DatabaseError> for JsValue {
    fn from(e: DatabaseError) -> Self {
        JsValue::from_str(&e.to_string())
    }
}
