use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::Stream;
use http::HeaderMap;
use send_wrapper::SendWrapper;
use serde::de::DeserializeOwned;

use crate::{
    body::{wasm::WasmStreamBody, HttpBody},
    futures::SendJsFuture,
    Error,
};

type BoxBody = http_body::combinators::UnsyncBoxBody<Bytes, Error>;

fn try_downcast<T, K>(k: K) -> Result<T, K>
where
    T: 'static,
    K: Send + 'static,
{
    let mut k = Some(k);
    if let Some(k) = <dyn std::any::Any>::downcast_mut::<Option<T>>(&mut k) {
        Ok(k.take().unwrap())
    } else {
        Err(k.unwrap())
    }
}

#[derive(Debug)]
pub(crate) enum BodyInner {
    None,
    BoxBody(BoxBody),
    WebSysRequest(SendWrapper<web_sys::Request>),
    WebSysResponse(SendWrapper<web_sys::Response>),
}

/// The body type used in requests and responses.
#[derive(Debug)]
pub struct Body(BodyInner);

impl Body {
    /// Create a new `Body` from a [`http_body::Body`].
    ///
    /// # Example
    ///
    /// ```
    /// # use worker::body::Body;
    /// let body = http_body::Full::from("hello world");
    /// let body = Body::new(body);
    /// ```
    pub fn new<B>(body: B) -> Self
    where
        B: HttpBody<Data = Bytes> + Send + 'static,
    {
        if body
            .size_hint()
            .exact()
            .map(|size| size == 0)
            .unwrap_or_default()
        {
            return Self::empty();
        }

        try_downcast(body).unwrap_or_else(|body| {
            Self(BodyInner::BoxBody(
                body.map_err(|_| Error::BadEncoding).boxed_unsync(),
            ))
        })
    }

    /// Create an empty body.
    pub const fn empty() -> Self {
        Self(BodyInner::None)
    }

    /// Get the full body as `Bytes`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn run() -> Result<(), worker::Error> {
    /// # use worker::body::Body;
    /// let body = Body::from("hello world");
    /// let bytes = body.bytes().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bytes(self) -> Result<Bytes, Error> {
        async fn array_buffer_to_bytes(
            buf: Result<js_sys::Promise, wasm_bindgen::JsValue>,
        ) -> Bytes {
            // Unwrapping only panics when the body has already been accessed before
            let fut = SendJsFuture::from(buf.unwrap());
            let buf = js_sys::Uint8Array::new(&fut.await.unwrap());
            buf.to_vec().into()
        }

        // Check the type of the body we have. Using the `array_buffer` function on the JS types might improve
        // performance as there's no polling overhead.
        match self.0 {
            BodyInner::None => Ok(Bytes::new()),
            BodyInner::BoxBody(body) => super::to_bytes(body).await,
            BodyInner::WebSysRequest(req) => Ok(array_buffer_to_bytes(req.array_buffer()).await),
            BodyInner::WebSysResponse(res) => Ok(array_buffer_to_bytes(res.array_buffer()).await),
        }
    }

    /// Get the full body as UTF-8.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn run() -> Result<(), worker::Error> {
    /// # use worker::body::Body;
    /// let body = Body::from("hello world");
    /// let text = body.text().await?;
    /// # Ok(())
    /// # }
    pub async fn text(self) -> Result<String, Error> {
        // JS strings are UTF-16 so using the JS function for `text` would introduce unnecessary overhead
        self.bytes()
            .await
            .and_then(|buf| String::from_utf8(buf.to_vec()).map_err(|_| Error::BadEncoding))
    }

    /// Get the full body as JSON.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn run() -> Result<(), worker::Error> {
    /// # use bytes::Bytes;
    /// # use serde::Deserialize;
    /// # use worker::body::Body;
    /// #[derive(Deserialize)]
    /// struct Ip {
    ///     origin: String,
    /// }
    ///
    /// let body = Body::from(r#"{"origin":"127.0.0.1"}"#);
    /// let ip = body.json::<Ip>().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn json<B: DeserializeOwned>(self) -> Result<B, Error> {
        self.bytes()
            .await
            .and_then(|buf| serde_json::from_slice(&buf).map_err(Error::SerdeJsonError))
    }

    pub(crate) fn is_none(&self) -> bool {
        match &self.0 {
            BodyInner::None => true,
            BodyInner::BoxBody(_) => false,
            BodyInner::WebSysRequest(req) => req.body().is_none(),
            BodyInner::WebSysResponse(res) => res.body().is_none(),
        }
    }

    /// Turns the body into a regular streaming body, if it's not already, and returns the underlying body.
    fn as_inner_box_body(&mut self) -> Option<&mut BoxBody> {
        match &self.0 {
            BodyInner::WebSysRequest(req) => *self = req.body().map(WasmStreamBody::new).into(),
            BodyInner::WebSysResponse(res) => *self = res.body().map(WasmStreamBody::new).into(),
            _ => {}
        }

        match &mut self.0 {
            BodyInner::None => None,
            BodyInner::BoxBody(body) => Some(body),
            _ => unreachable!(),
        }
    }
}

impl Default for Body {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<()> for Body {
    fn from(_: ()) -> Self {
        Self::empty()
    }
}

impl<B> From<Option<B>> for Body
where
    B: HttpBody<Data = Bytes> + Send + 'static,
{
    fn from(body: Option<B>) -> Self {
        body.map(Body::new).unwrap_or_else(Self::empty)
    }
}

impl From<web_sys::Request> for Body {
    fn from(req: web_sys::Request) -> Self {
        Self(BodyInner::WebSysRequest(SendWrapper::new(req)))
    }
}

impl From<web_sys::Response> for Body {
    fn from(res: web_sys::Response) -> Self {
        Self(BodyInner::WebSysResponse(SendWrapper::new(res)))
    }
}

macro_rules! body_from_impl {
    ($ty:ty) => {
        impl From<$ty> for Body {
            fn from(buf: $ty) -> Self {
                Self::new(http_body::Full::from(buf))
            }
        }
    };
}

body_from_impl!(&'static [u8]);
body_from_impl!(std::borrow::Cow<'static, [u8]>);
body_from_impl!(Vec<u8>);

body_from_impl!(&'static str);
body_from_impl!(std::borrow::Cow<'static, str>);
body_from_impl!(String);

body_from_impl!(Bytes);

impl HttpBody for Body {
    type Data = Bytes;
    type Error = Error;

    #[inline]
    fn poll_data(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        match self.as_inner_box_body() {
            Some(body) => Pin::new(body).poll_data(cx),
            None => Poll::Ready(None),
        }
    }

    #[inline]
    fn poll_trailers(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<HeaderMap>, Self::Error>> {
        match self.as_inner_box_body() {
            Some(body) => Pin::new(body).poll_trailers(cx),
            None => Poll::Ready(Ok(None)),
        }
    }

    #[inline]
    fn size_hint(&self) -> http_body::SizeHint {
        match &self.0 {
            BodyInner::None => http_body::SizeHint::with_exact(0),
            BodyInner::BoxBody(body) => body.size_hint(),
            BodyInner::WebSysRequest(_) => http_body::SizeHint::new(),
            BodyInner::WebSysResponse(_) => http_body::SizeHint::new(),
        }
    }

    #[inline]
    fn is_end_stream(&self) -> bool {
        match &self.0 {
            BodyInner::None => true,
            BodyInner::BoxBody(body) => body.is_end_stream(),
            BodyInner::WebSysRequest(_) => false,
            BodyInner::WebSysResponse(_) => false,
        }
    }
}

impl Stream for Body {
    type Item = Result<Bytes, Error>;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_data(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static_assertions::assert_impl_all!(Body: Send, Unpin);
}