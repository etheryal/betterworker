use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::stream::FusedStream;
use futures_util::{Stream, StreamExt};
use http::HeaderMap;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_streams::readable::IntoStream;

use crate::error::WorkerError;

/// Body wrapping a JS `ReadableStream`.
pub(super) struct WasmStreamBody(SendWrapper<IntoStream<'static>>);

impl WasmStreamBody {
    pub fn new(stream: web_sys::ReadableStream) -> Self {
        let stream = wasm_streams::ReadableStream::from_raw(stream.unchecked_into()).into_stream();
        Self(SendWrapper::new(stream))
    }
}

impl http_body::Body for WasmStreamBody {
    type Data = Bytes;
    type Error = WorkerError;

    #[inline]
    fn poll_data(
        mut self: Pin<&mut Self>, cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        self.0
            .poll_next_unpin(cx)
            .map_ok(|buf| js_sys::Uint8Array::from(buf).to_vec().into())
            .map_err(WorkerError::from_js_err)
    }

    #[inline]
    fn poll_trailers(
        self: Pin<&mut Self>, _cx: &mut Context<'_>,
    ) -> Poll<Result<Option<HeaderMap>, Self::Error>> {
        Poll::Ready(Ok(None))
    }

    #[inline]
    fn size_hint(&self) -> http_body::SizeHint {
        let (lower, upper) = self.0.size_hint();

        let mut hint = http_body::SizeHint::new();
        hint.set_lower(lower as u64);
        if let Some(upper) = upper {
            hint.set_upper(upper as u64);
        }

        hint
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_terminated()
    }
}
