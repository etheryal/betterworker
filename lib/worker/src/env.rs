use std::convert::TryFrom;

use betterworker_sys::Env as EnvSys;
use js_sys::{JsString, Object};
use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, JsValue};
use worker_kv::KvStore;

#[cfg(feature = "d1")]
use crate::d1::Database;
use crate::durable::ObjectNamespace;
use crate::dynamic_dispatch::DynamicDispatcher;
use crate::error::Error;
use crate::fetcher::Fetcher;
use crate::prelude::Bucket;
#[cfg(feature = "queue")]
use crate::queue::Queue;
use crate::result::Result;

/// Env contains any bindings you have associated with the Worker when you
/// uploaded it.
#[derive(Clone)]
pub struct Env(SendWrapper<EnvSys>);

impl From<EnvSys> for Env {
    fn from(env: EnvSys) -> Self {
        Self(SendWrapper::new(env))
    }
}

impl Env {
    fn get_binding<T: TryFrom<Object, Error = Error>>(&self, name: &str) -> Result<T> {
        let binding = js_sys::Reflect::get(self.0.as_ref(), &JsValue::from(name))
            .map_err(|_| Error::EnvBindingError(name.to_string()))?;
        if binding.is_undefined() {
            Err(Error::UndefinedBinding(name.to_string()))
        } else {
            let object = Object::from(binding);
            T::try_from(object)
        }
    }

    /// Access Secret value bindings added to your Worker via the UI or
    /// `wrangler`: <https://developers.cloudflare.com/workers/cli-wrangler/commands#secret>
    pub fn secret(&self, binding: &str) -> Result<Secret> {
        self.get_binding::<Secret>(binding)
    }

    /// Environment variables are defined via the `[vars]` configuration in your
    /// wrangler.toml file and are always plaintext values.
    pub fn var(&self, binding: &str) -> Result<Var> {
        self.get_binding::<Var>(binding)
    }

    /// Access a Workers KV namespace by the binding name configured in your
    /// wrangler.toml file.
    pub fn kv(&self, binding: &str) -> Result<KvStore> {
        KvStore::from_this(self.0.as_ref(), binding).map_err(From::from)
    }

    /// Access a Durable Object namespace by the binding name configured in your
    /// wrangler.toml file.
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
    /// Access a Queue by the binding name configured in your wrangler.toml
    /// file.
    pub fn queue(&self, binding: &str) -> Result<Queue> {
        self.get_binding(binding)
    }

    /// Access an R2 Bucket by the binding name configured in your wrangler.toml
    /// file.
    pub fn bucket(&self, binding: &str) -> Result<Bucket> {
        self.get_binding(binding)
    }

    /// Access a D1 Database by the binding name configured in your
    /// wrangler.toml file.
    #[cfg(feature = "d1")]
    pub fn d1(&self, binding: &str) -> Result<Database> {
        self.get_binding(binding)
    }

    #[doc(hidden)]
    pub fn _inner(self) -> EnvSys {
        self.0.take()
    }
}

pub struct StringBinding(String);

impl TryFrom<Object> for StringBinding {
    type Error = Error;

    fn try_from(obj: Object) -> Result<Self> {
        let js_string = obj.dyn_into::<JsString>().map_err(|obj| {
            let name = obj.constructor().name().as_string().unwrap();
            Error::BindingCast(name, "String".into())
        })?;
        Ok(Self(js_string.into()))
    }
}

impl ToString for StringBinding {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl AsRef<str> for StringBinding {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A string value representing a binding to a secret in a Worker.
pub type Secret = StringBinding;

/// A string value representing a binding to an environment variable in a
/// Worker.
pub type Var = StringBinding;
