#![deny(unsafe_code)]

#[cfg(feature = "d1")]
pub use betterworker_d1 as d1;
pub use betterworker_macros::{durable_object, event};
#[doc(hidden)]
pub use betterworker_sys;
#[doc(hidden)]
pub use js_sys;
pub use url::Url;
#[doc(hidden)]
pub use wasm_bindgen;
#[doc(hidden)]
pub use wasm_bindgen_futures;
pub use worker_kv as kv;

pub mod abort;
pub mod body;
pub mod cache;
pub mod cf;
pub mod context;
pub mod date;
pub mod delay;
pub mod durable;
pub mod dynamic_dispatch;
pub mod env;
pub mod error;
pub mod fetch;
pub mod fetcher;
pub mod http;
pub mod prelude;
#[cfg(feature = "queue")]
pub mod queue;
pub mod r2;
pub mod result;
pub mod schedule;
pub mod socket;
pub mod streams;
pub mod websocket;

mod futures;

#[cfg(test)]
mod test_assertions;
