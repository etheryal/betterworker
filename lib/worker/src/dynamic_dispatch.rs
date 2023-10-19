use std::convert::TryFrom;

use betterworker_sys::DynamicDispatcher as DynamicDispatcherSys;
use js_sys::Object;
use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, JsValue};

use crate::error::WorkerError;
use crate::fetcher::Fetcher;
use crate::result::Result;

/// A binding for dispatching events to Workers inside of a dispatch namespace
/// by their name. This allows for your worker to directly invoke many workers
/// by name instead of having multiple service worker bindings.
///
/// # Example:
///
/// ```ignore
/// let dispatcher = env.dynamic_dispatcher("DISPATCHER")?;
/// let fetcher = dispatcher.get("namespaced-worker-name")?;
/// let resp = fetcher.fetch_request(req).await?;
/// ```
#[derive(Debug, Clone)]
pub struct DynamicDispatcher(SendWrapper<DynamicDispatcherSys>);

impl DynamicDispatcher {
    /// Gets a [Fetcher] for a Worker inside of the dispatch namespace based of
    /// the name specified.
    pub fn get(&self, name: impl Into<String>) -> Result<Fetcher> {
        let fetcher_sys = self
            .0
            .get(name.into(), JsValue::undefined())
            .map_err(WorkerError::from_js_err)?;
        Ok(fetcher_sys.into())
    }
}

impl AsRef<JsValue> for DynamicDispatcher {
    fn as_ref(&self) -> &wasm_bindgen::JsValue {
        &self.0
    }
}

impl TryFrom<Object> for DynamicDispatcher {
    type Error = WorkerError;

    fn try_from(obj: Object) -> Result<Self> {
        const TYPE_NAME: &'static str = "DynamicDispatcher";

        let data = if obj.constructor().name() == TYPE_NAME {
            obj.unchecked_into()
        } else {
            return Err(WorkerError::InvalidBinding);
        };
        Ok(Self(SendWrapper::new(data)))
    }
}

impl From<DynamicDispatcher> for JsValue {
    fn from(ns: DynamicDispatcher) -> Self {
        JsValue::from(ns.0.take())
    }
}
