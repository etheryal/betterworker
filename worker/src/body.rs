//! Body types and functions

#[allow(clippy::module_inception)]
mod body;
mod to_bytes;
mod wasm;

pub use body::Body;
pub use http_body::Body as HttpBody;
pub use to_bytes::to_bytes;

pub use bytes::{Buf, BufMut, Bytes};
