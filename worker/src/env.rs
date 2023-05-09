use std::convert::TryFrom;

#[cfg(feature = "d1")]
use crate::d1::Database;
use crate::error::Error;
#[cfg(feature = "queue")]
use crate::Queue;
use crate::{durable::ObjectNamespace, Bucket, DynamicDispatcher, Fetcher, Result};

use js_sys::JsString;
use send_wrapper::SendWrapper;
use wasm_bindgen::{prelude::*, JsCast, JsValue};
use worker_kv::KvStore;

#[wasm_bindgen]
extern "C" {
    /// Env contains any bindings you have associated with the Worker when you uploaded it.
    pub type Env;
}

unsafe impl Send for Env {}
unsafe impl Sync for Env {}

impl Env {
    fn get_binding<T: TryFrom<JsValue, Error = Error>>(&self, name: &str) -> Result<T> {
        let binding = js_sys::Reflect::get(self, &JsValue::from(name))
            .map_err(|_| Error::JsError(format!("Env does not contain binding `{name}`")))?;
        if binding.is_undefined() {
            Err(format!("Binding `{name}` is undefined.").into())
        } else {
            T::try_from(binding)
        }
    }

    /// Access Secret value bindings added to your Worker via the UI or `wrangler`:
    /// <https://developers.cloudflare.com/workers/cli-wrangler/commands#secret>
    pub fn secret(&self, binding: &str) -> Result<Secret> {
        self.get_binding::<Secret>(binding)
    }

    /// Environment variables are defined via the `[vars]` configuration in your wrangler.toml file
    /// and are always plaintext values.
    pub fn var(&self, binding: &str) -> Result<Var> {
        self.get_binding::<Var>(binding)
    }

    /// Access a Workers KV namespace by the binding name configured in your wrangler.toml file.
    pub fn kv(&self, binding: &str) -> Result<KvStore> {
        KvStore::from_this(self, binding).map_err(From::from)
    }

    /// Access a Durable Object namespace by the binding name configured in your wrangler.toml file.
    pub fn durable_object(&self, binding: &str) -> Result<ObjectNamespace> {
        self.get_binding(binding)
    }

    /// Access a Dynamic Dispatcher for dispatching events to other workers.
    pub fn dynamic_dispatcher(&self, binding: &str) -> Result<DynamicDispatcher> {
        self.get_binding(binding)
    }

    /// Get a [Service Binding](https://developers.cloudflare.com/workers/runtime-apis/service-bindings/)
    /// for Worker-to-Worker communication.
    pub fn service(&self, binding: &str) -> Result<Fetcher> {
        self.get_binding(binding)
    }

    #[cfg(feature = "queue")]
    /// Access a Queue by the binding name configured in your wrangler.toml file.
    pub fn queue(&self, binding: &str) -> Result<Queue> {
        self.get_binding(binding)
    }

    /// Access an R2 Bucket by the binding name configured in your wrangler.toml file.
    pub fn bucket(&self, binding: &str) -> Result<Bucket> {
        self.get_binding(binding)
    }

    /// Access a D1 Database by the binding name configured in your wrangler.toml file.
    #[cfg(feature = "d1")]
    pub fn d1(&self, binding: &str) -> Result<Database> {
        self.get_binding(binding)
    }
}

pub struct StringBinding(SendWrapper<JsString>);

impl AsRef<JsValue> for StringBinding {
    fn as_ref(&self) -> &wasm_bindgen::JsValue {
        self.0.as_ref()
    }
}

impl TryFrom<JsValue> for StringBinding {
    type Error = Error;

    fn try_from(val: JsValue) -> Result<Self> {
        let data = val.dyn_into()?;
        Ok(StringBinding(SendWrapper::new(data)))
    }
}

impl From<StringBinding> for JsValue {
    fn from(sec: StringBinding) -> Self {
        sec.0.take().into()
    }
}

impl ToString for StringBinding {
    fn to_string(&self) -> String {
        self.0.as_string().unwrap_or_default()
    }
}

/// A string value representing a binding to a secret in a Worker.
pub type Secret = StringBinding;
/// A string value representing a binding to an environment variable in a Worker.
pub type Var = StringBinding;
