use betterworker_sys::ext::{AbortControllerExt, AbortSignalExt};
use send_wrapper::SendWrapper;
use wasm_bindgen::JsValue;

/// An interface that allows you to abort in-flight [Fetch](crate::Fetch)
/// requests.
#[derive(Debug)]
pub struct AbortController(SendWrapper<web_sys::AbortController>);

impl AbortController {
    /// Gets a [AbortSignal] which can be passed to a cancellable operation.
    pub fn signal(&self) -> AbortSignal {
        AbortSignal(SendWrapper::new(self.0.signal()))
    }

    /// Aborts any operation using a [AbortSignal] created from this controller.
    pub fn abort(self) {
        self.0.abort()
    }

    /// Aborts any operation using a [AbortSignal] created from this controller
    /// with the provided reason.
    pub fn abort_with_reason(self, reason: impl Into<JsValue>) {
        self.0.abort_with_reason(&reason.into())
    }
}

impl Default for AbortController {
    fn default() -> Self {
        Self(SendWrapper::new(web_sys::AbortController::new().unwrap()))
    }
}

/// An interface representing a signal that can be passed to cancellable
/// operations, primarily a [Fetch](crate::Fetch) request.
#[derive(Debug, Clone)]
pub struct AbortSignal(SendWrapper<web_sys::AbortSignal>);

impl AbortSignal {
    /// A [bool] indicating if the operation that the signal is used for has
    /// been aborted.
    pub fn aborted(&self) -> bool {
        self.0.aborted()
    }

    /// The reason why the signal was aborted.
    pub fn reason(&self) -> Option<JsValue> {
        self.aborted().then(|| self.0.reason())
    }

    /// Creates a [AbortSignal] that is already aborted.
    pub fn abort() -> Self {
        Self(SendWrapper::new(web_sys::AbortSignal::abort()))
    }

    /// Creates a [AbortSignal] that is already aborted with the provided
    /// reason.
    pub fn abort_with_reason(reason: impl Into<JsValue>) -> Self {
        Self(SendWrapper::new(web_sys::AbortSignal::abort_with_reason(
            &reason.into(),
        )))
    }

    pub(crate) fn inner(&self) -> &web_sys::AbortSignal {
        &self.0
    }
}

impl From<web_sys::AbortSignal> for AbortSignal {
    fn from(signal: web_sys::AbortSignal) -> Self {
        Self(SendWrapper::new(signal))
    }
}
