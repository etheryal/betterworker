use wasm_bindgen::JsCast;
use web_sys::WorkerGlobalScope;

use crate::{
    body::Body,
    futures::SendJsFuture,
    http::{request, response},
    Error, Result,
};

/// Fetch a resource from the network.
///
/// # Example
///
/// ```rust,ignore
/// # async fn run() -> worker::Result<()> {
/// use worker::fetch;
///
/// let req = http::Request::get("https://www.rust-lang.org/")
///     .body(())
///     .unwrap();
///
/// let res = fetch(req)
///     .await?
///     .into_body()
///     .text()
///     .await?;
///
/// println!("{res}");
/// # Ok(())
/// # }
/// ```
pub async fn fetch(req: http::Request<impl Into<Body>>) -> Result<http::Response<Body>> {
    let fut = {
        let req = req.map(Into::into);
        let global = js_sys::global().unchecked_into::<WorkerGlobalScope>();

        let req = request::into_web_sys_request(req);
        let promise = global.fetch_with_request(&req);

        SendJsFuture::from(promise)
    };

    fut.await
        .map(|res| response::from_web_sys_response(res.unchecked_into()))
        .map_err(Error::from)
}
