use thiserror::Error;
use wasm_bindgen::{JsCast, JsValue};

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("content-type mismatch")]
    BadEncoding,

    #[error("body has already been read")]
    BodyUsed,

    #[error("Env does not contain binding `{0}`")]
    EnvBindingError(String),

    #[error("invalid range")]
    InvalidRange,

    #[error("route has no corresponding shared data")]
    RouteNoDataError,

    #[error("Binding `{0}` is undefined.")]
    UndefinedBinding(String),

    #[error("Binding cannot be cast to the type {0} from {1}")]
    BindingCast(String, String),

    #[error("Must pass in a struct type")]
    MustPassInStructType,

    #[error("fixed length stream had different length than expected (expected {0}, got {1})")]
    FixedLengthStreamLengthError(u64, u64),

    #[error("data of message event is not text")]
    MessageEventNotText,

    #[error("server did not accept websocket connection")]
    WebSocketConnectionError,

    #[error(transparent)]
    RouteInsertError(#[from] matchit::InsertError),

    #[error(transparent)]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[cfg(feature = "queue")]
    #[error("serde_wasm_bindgen error")]
    SerdeWasmBindgenError(send_wrapper::SendWrapper<serde_wasm_bindgen::Error>),

    #[error("{0}")]
    Custom(String),
}

impl From<JsValue> for Error {
    fn from(v: JsValue) -> Self {
        let message = v
            .as_string()
            .or_else(|| {
                v.dyn_ref::<js_sys::Error>().map(|e| {
                    format!(
                        "{} Message: {} Cause: {:?}",
                        e.to_string(),
                        e.message(),
                        e.cause()
                    )
                })
            })
            .unwrap_or_else(|| format!("Unknown Javascript error: {:?}", v));
        Self::Custom(message)
    }
}

impl From<worker_kv::KvError> for Error {
    fn from(e: worker_kv::KvError) -> Self {
        let val: JsValue = e.into();
        val.into()
    }
}

impl From<serde_wasm_bindgen::Error> for Error {
    fn from(e: serde_wasm_bindgen::Error) -> Self {
        let val: JsValue = e.into();
        val.into()
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::Custom(s)
    }
}

impl From<Error> for JsValue {
    fn from(e: Error) -> Self {
        JsValue::from_str(&e.to_string())
    }
}
