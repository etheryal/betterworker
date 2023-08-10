//! HTTP types and functions

mod redirect;
pub mod request;
pub mod response;

pub use http::*;
pub use redirect::RequestRedirect;
