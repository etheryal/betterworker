use std::convert::TryFrom;

use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, JsValue};
use worker_sys::DynamicDispatcher as DynamicDispatcherSys;

use crate::{Fetcher, Result};

/// A binding for dispatching events to Workers inside of a dispatch namespace by their name. This
/// allows for your worker to directly invoke many workers by name instead of having multiple
/// service worker bindings.
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
    /// Gets a [Fetcher] for a Worker inside of the dispatch namespace based of the name specified.
    pub fn get(&self, name: impl Into<String>) -> Result<Fetcher> {
        let fetcher_sys = self.0.get(name.into(), JsValue::undefined())?;
        Ok(fetcher_sys.into())
    }
}

impl AsRef<JsValue> for DynamicDispatcher {
    fn as_ref(&self) -> &wasm_bindgen::JsValue {
        &self.0
    }
}

impl TryFrom<JsValue> for DynamicDispatcher {
    type Error = crate::Error;

    fn try_from(val: JsValue) -> Result<Self> {
        Ok(Self(SendWrapper::new(val.dyn_into()?)))
    }
}

impl From<DynamicDispatcher> for JsValue {
    fn from(ns: DynamicDispatcher) -> Self {
        JsValue::from(ns.0.take())
    }
}
