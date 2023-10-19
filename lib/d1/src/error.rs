use serde_wasm_bindgen::Error as SerdeWasmBindgenError;
use thiserror::Error;
use wasm_bindgen::JsValue;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DatabaseError {
    #[error("D1 query error")]
    Query,

    #[error("Env binding is invalid")]
    InvalidBinding,

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

impl From<SerdeWasmBindgenError> for DatabaseError {
    fn from(e: SerdeWasmBindgenError) -> Self {
        DatabaseError::SerdeWasmBindgen(format!("{}", e))
    }
}

impl From<DatabaseError> for JsValue {
    fn from(e: DatabaseError) -> Self {
        JsValue::from_str(&e.to_string())
    }
}
