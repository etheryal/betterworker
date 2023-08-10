//! Functions for translating responses to and from JS

use betterworker_sys::ext::{HeadersExt, ResponseExt, ResponseInitExt};
use wasm_bindgen::JsCast;

use crate::{body::Body, prelude::WebSocket};

/// Create a [`http::Response`] from a [`web_sys::Response`].
///
/// # Extensions
///
/// The following types may be added in the [`Extensions`] of the `Response`.
///
/// - [`WebSocket`]
///
/// # Example
///
/// ```rust,ignore
/// use betterworker::http::response;
///
/// let res = web_sys::Response::new_with_opt_str(Some("hello world")).unwrap();
/// let res = response::from_wasm(res);
///
/// println!("{}", res.status());
/// ```
///
/// [`Extensions`]: http::Extensions
pub fn from_web_sys_response(res: web_sys::Response) -> http::Response<Body> {
    let mut builder = http::Response::builder().status(res.status());

    for header in res.headers().entries() {
        let header = header.unwrap().unchecked_into::<js_sys::Array>();
        builder = builder.header(
            header.get(0).as_string().unwrap(),
            header.get(1).as_string().unwrap(),
        );
    }

    if let Some(ws) = res.websocket() {
        builder = builder.extension(WebSocket::from(ws));
    }

    builder.body(Body::from(res)).unwrap()
}

/// Create a [`web_sys::Response`] from a [`http::Response`].
///
/// # Extensions
///
/// The following types may be added in the [`Extensions`] of the `Response`.
///
/// - [`WebSocket`]
///
/// # Example
///
/// ```rust,ignore
/// use betterworker::body::Body;
/// use betterworker::http::response;
///
/// let res = http::Response::new(Body::from("hello world"));
/// let res = response::into_wasm(res);
///
/// println!("{}", res.status());
/// ```
///
/// [`Extensions`]: http::Extensions
pub fn into_web_sys_response(mut res: http::Response<Body>) -> web_sys::Response {
    let status = res.status().as_u16();

    let headers = web_sys::Headers::new().unwrap();
    for (name, value) in res
        .headers()
        .into_iter()
        .filter_map(|(name, value)| value.to_str().map(|value| (name.as_str(), value)).ok())
    {
        headers.append(name, value).unwrap();
    }

    let mut init = web_sys::ResponseInit::new();
    init.status(status).headers(&headers);

    if let Some(ws) = res.extensions_mut().remove::<WebSocket>() {
        init.websocket(ws.as_ref());
    }

    let stream = res.into_body().into_stream();
    web_sys::Response::new_with_opt_readable_stream_and_init(stream.as_ref(), &init).unwrap()
}
