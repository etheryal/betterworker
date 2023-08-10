use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::{Future, FutureExt};
use js_sys::Promise;
use send_wrapper::SendWrapper;
use wasm_bindgen_futures::JsFuture;

/// [`JsFuture`] that is explicitely [`Send`].
pub(crate) struct SendJsFuture(SendWrapper<JsFuture>);

impl Future for SendJsFuture {
    type Output = <JsFuture as Future>::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_unpin(cx)
    }
}

impl From<Promise> for SendJsFuture {
    fn from(p: Promise) -> Self {
        Self(SendWrapper::new(JsFuture::from(p)))
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_impl_all;

    use super::*;

    assert_impl_all!(SendJsFuture: Send, Sync, Unpin);
}
