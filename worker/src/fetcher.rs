use std::convert::TryFrom;

use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, JsValue};

use crate::{
    body::Body,
    futures::SendJsFuture,
    http::{request, response},
    Result,
};

/// A struct for invoking fetch events to other Workers.
pub struct Fetcher(SendWrapper<worker_sys::Fetcher>);

impl Fetcher {
    /// Invoke a fetch event in a worker with a url and optionally a [RequestInit].
    pub async fn fetch(&self, req: http::Request<Body>) -> Result<http::Response<Body>> {
        let fut = {
            let req = request::into_wasm(req);
            let promise = self.0.fetch(&req);

            SendJsFuture::from(promise)
        };

        let res = fut.await?.dyn_into()?;
        Ok(response::from_wasm(res))
    }
}


impl AsRef<wasm_bindgen::JsValue> for Fetcher {
    fn as_ref(&self) -> &wasm_bindgen::JsValue {
        &self.0
    }
}

impl From<worker_sys::Fetcher> for Fetcher {
    fn from(inner: worker_sys::Fetcher) -> Self {
        Self(SendWrapper::new(inner))
    }
}

impl TryFrom<JsValue> for Fetcher {
    type Error = crate::Error;

    fn try_from(val: JsValue) -> Result<Self> {
        Ok(Self(SendWrapper::new(val.dyn_into()?)))
    }
}

impl From<Fetcher> for JsValue {
    fn from(ns: Fetcher) -> Self {
        JsValue::from(ns.0.take())
    }
}