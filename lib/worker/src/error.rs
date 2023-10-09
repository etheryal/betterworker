use thiserror::Error;
use wasm_bindgen::{JsCast, JsValue};

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum WorkerError {
    #[error("content-type mismatch")]
    BadEncoding,

    #[error("body has already been read")]
    BodyUsed,

    #[error("Failed to obtain environment binding `{0}`")]
    EnvBindingError(String),

    #[error("invalid range")]
    InvalidRange,

    #[error("Binding `{0}` is undefined.")]
    UndefinedBinding(String),

    #[error("Binding cannot be cast")]
    BindingCast,

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

    #[error("serde-wasm-bindgen error: {0}")]
    SerdeWasmBindgenError(String),

    #[error("worker-kv error: {0}")]
    WorkerKvError(String),

    #[error("await promise error: {0}")]
    AwaitPromise(String),

    #[error("javascript error: {0}")]
    JsError(String),

    #[error("Failed to cast a JsValue")]
    JsCast,

    #[error("Cannot get stub from within a Durable Object")]
    DurableObjectStub,

    #[error("Invalid message batch. Failed to get id from message.")]
    InvalidMessageBatch,

    #[error(transparent)]
    #[cfg(feature = "d1")]
    D1Error(#[from] betterworker_d1::error::DatabaseError),
}

impl WorkerError {
    pub(crate) fn from_promise_err(err: JsValue) -> Self {
        let message = err
            .as_string()
            .or_else(|| {
                err.dyn_ref::<js_sys::Error>().map(|e| {
                    format!(
                        "{} Message: {} Cause: {:?}",
                        e.to_string(),
                        e.message(),
                        e.cause()
                    )
                })
            })
            .unwrap_or_else(|| format!("Unknown Javascript error: {:?}", err));
        Self::AwaitPromise(message)
    }

    pub(crate) fn from_js_err(err: JsValue) -> Self {
        let message = err
            .as_string()
            .or_else(|| {
                err.dyn_ref::<js_sys::Error>().map(|e| {
                    format!(
                        "{} Message: {} Cause: {:?}",
                        e.to_string(),
                        e.message(),
                        e.cause()
                    )
                })
            })
            .unwrap_or_else(|| format!("Unknown Javascript error: {:?}", err));
        Self::JsError(message)
    }

    pub(crate) fn from_cast_err(_: JsValue) -> Self {
        Self::JsCast
    }
}

impl From<worker_kv::KvError> for WorkerError {
    fn from(e: worker_kv::KvError) -> Self {
        let val: JsValue = e.into();
        let message = val
            .as_string()
            .or_else(|| {
                val.dyn_ref::<js_sys::Error>().map(|e| {
                    format!(
                        "{} Message: {} Cause: {:?}",
                        e.to_string(),
                        e.message(),
                        e.cause()
                    )
                })
            })
            .unwrap_or_else(|| format!("Unknown worker-kv error: {:?}", val));
        WorkerError::WorkerKvError(message)
    }
}

impl From<serde_wasm_bindgen::Error> for WorkerError {
    fn from(e: serde_wasm_bindgen::Error) -> Self {
        let val: JsValue = e.into();
        let message = val
            .as_string()
            .or_else(|| {
                val.dyn_ref::<js_sys::Error>().map(|e| {
                    format!(
                        "{} Message: {} Cause: {:?}",
                        e.to_string(),
                        e.message(),
                        e.cause()
                    )
                })
            })
            .unwrap_or_else(|| format!("Unknown serde-wasm-bindgen error: {:?}", val));
        WorkerError::SerdeWasmBindgenError(message)
    }
}

impl From<WorkerError> for JsValue {
    fn from(e: WorkerError) -> Self {
        JsValue::from_str(&e.to_string())
    }
}
