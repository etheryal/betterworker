use std::convert::TryFrom;

use betterworker_sys::Fetcher as FetcherSys;
use js_sys::Object;
use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, JsValue};

use crate::body::Body;
use crate::error::WorkerError;
use crate::futures::SendJsFuture;
use crate::http::{request, response};
use crate::result::Result;

/// A struct for invoking fetch events to other Workers.
pub struct Fetcher(SendWrapper<FetcherSys>);

impl Fetcher {
    /// Invoke a fetch event in a worker with a url and optionally a
    /// [RequestInit].
    pub async fn fetch(&self, req: http::Request<Body>) -> Result<http::Response<Body>> {
        let fut = {
            let req = request::into_web_sys_request(req);
            let promise = self.0.fetch(&req);

            SendJsFuture::from(promise)
        };

        let promise = fut.await.map_err(WorkerError::from_promise_err)?;
        let res = promise.dyn_into().map_err(WorkerError::from_cast_err)?;
        Ok(response::from_web_sys_response(res))
    }
}

impl AsRef<wasm_bindgen::JsValue> for Fetcher {
    fn as_ref(&self) -> &wasm_bindgen::JsValue {
        &self.0
    }
}

impl From<FetcherSys> for Fetcher {
    fn from(inner: FetcherSys) -> Self {
        Self(SendWrapper::new(inner))
    }
}

impl TryFrom<Object> for Fetcher {
    type Error = WorkerError;

    fn try_from(obj: Object) -> Result<Self> {
        const TYPE_NAME: &'static str = "Fetcher";

        let data = if obj.constructor().name() == TYPE_NAME {
            obj.unchecked_into()
        } else {
            return Err(WorkerError::InvalidBinding);
        };
        Ok(Self(SendWrapper::new(data)))
    }
}

impl From<Fetcher> for JsValue {
    fn from(ns: Fetcher) -> Self {
        JsValue::from(ns.0.take())
    }
}
