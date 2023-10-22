use std::pin::Pin;

use futures_util::Future;
use js_sys::Promise;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

pub(crate) fn future_from_promise(
    p: Promise,
) -> Pin<Box<dyn Future<Output = Result<JsValue, JsValue>> + Send + Sync + 'static>> {
    Box::pin(SendWrapper::new(JsFuture::from(p)))
}
