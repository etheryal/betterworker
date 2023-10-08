use std::convert::TryFrom;
use std::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, Iterator};
use std::marker::PhantomData;

use betterworker_sys::{MessageBatch as MessageBatchSys, Queue as EdgeQueue};
use js_sys::{Array, Object};
use send_wrapper::SendWrapper;
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::date::Date;
use crate::error::WorkerError;
use crate::futures::SendJsFuture;
use crate::result::Result;

static BODY_KEY_STR: &str = "body";
static ID_KEY_STR: &str = "id";
static TIMESTAMP_KEY_STR: &str = "timestamp";

struct MessageBatchInner<T> {
    inner: MessageBatchSys,
    messages: Array,
    data: PhantomData<T>,
    timestamp_key: JsValue,
    body_key: JsValue,
    id_key: JsValue,
}

pub struct MessageBatch<T>(SendWrapper<MessageBatchInner<T>>);

impl<T> MessageBatch<T> {
    pub fn new(message_batch_sys: MessageBatchSys) -> Self {
        let timestamp_key = JsValue::from_str(TIMESTAMP_KEY_STR);
        let body_key = JsValue::from_str(BODY_KEY_STR);
        let id_key = JsValue::from_str(ID_KEY_STR);
        let inner = MessageBatchInner {
            messages: message_batch_sys.messages(),
            inner: message_batch_sys,
            data: PhantomData,
            timestamp_key,
            body_key,
            id_key,
        };
        Self(SendWrapper::new(inner))
    }
}

pub struct Message<T> {
    pub body: T,
    pub timestamp: Date,
    pub id: String,
}

impl<T> MessageBatch<T> {
    /// The name of the Queue that belongs to this batch.
    pub fn queue(&self) -> String {
        self.0.inner.queue()
    }

    /// Marks every message to be retried in the next batch.
    pub fn retry_all(&self) {
        self.0.inner.retry_all();
    }

    /// Iterator that deserializes messages in the message batch. Ordering of
    /// messages is not guaranteed.
    pub fn iter(&self) -> MessageIter<'_, T>
    where
        T: DeserializeOwned, {
        let inner = MessageIterInner {
            range: 0..self.0.messages.length(),
            array: &self.0.messages,
            timestamp_key: &self.0.timestamp_key,
            body_key: &self.0.body_key,
            id_key: &self.0.id_key,
            data: PhantomData,
        };
        MessageIter(SendWrapper::new(inner))
    }

    /// An array of messages in the batch. Ordering of messages is not
    /// guaranteed.
    pub fn messages(&self) -> Result<Vec<Message<T>>>
    where
        T: DeserializeOwned, {
        self.iter().collect()
    }
}

struct MessageIterInner<'a, T> {
    range: std::ops::Range<u32>,
    array: &'a Array,
    timestamp_key: &'a JsValue,
    body_key: &'a JsValue,
    id_key: &'a JsValue,
    data: PhantomData<T>,
}

pub struct MessageIter<'a, T>(SendWrapper<MessageIterInner<'a, T>>);

impl<T> MessageIter<'_, T>
where
    T: DeserializeOwned,
{
    fn parse_message(&self, message: &JsValue) -> Result<Message<T>> {
        let raw_date = js_sys::Reflect::get(message, self.0.timestamp_key)
            .map_err(WorkerError::from_js_err)?;
        let date = js_sys::Date::from(raw_date);
        let id = js_sys::Reflect::get(message, self.0.id_key)
            .map_err(WorkerError::from_js_err)?
            .as_string()
            .ok_or(WorkerError::InvalidMessageBatch)?;
        let value = js_sys::Reflect::get(message, self.0.body_key).map_err(WorkerError::from_js_err)?;
        let body = serde_wasm_bindgen::from_value(value)?;

        Ok(Message {
            id,
            body,
            timestamp: Date::from(date),
        })
    }
}

impl<T> Iterator for MessageIter<'_, T>
where
    T: DeserializeOwned,
{
    type Item = Result<Message<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.0.range.next()?;
        let value = self.0.array.get(index);
        Some(self.parse_message(&value))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.range.size_hint()
    }
}

impl<T> DoubleEndedIterator for MessageIter<'_, T>
where
    T: DeserializeOwned,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let index = self.0.range.next_back()?;
        let value = self.0.array.get(index);

        Some(self.parse_message(&value))
    }
}

impl<'a, T> FusedIterator for MessageIter<'a, T> where T: DeserializeOwned {}

impl<'a, T> ExactSizeIterator for MessageIter<'a, T> where T: DeserializeOwned {}

pub struct Queue(SendWrapper<EdgeQueue>);

impl AsRef<JsValue> for Queue {
    fn as_ref(&self) -> &JsValue {
        self.0.as_ref()
    }
}

impl TryFrom<Object> for Queue {
    type Error = WorkerError;

    fn try_from(obj: Object) -> Result<Self> {
        const TYPE_NAME: &'static str = "Queue";

        let data = if obj.constructor().name() == TYPE_NAME {
            obj.unchecked_into()
        } else {
            return Err(WorkerError::BindingCast);
        };
        Ok(Self(SendWrapper::new(data)))
    }
}

impl From<Queue> for JsValue {
    fn from(ns: Queue) -> Self {
        JsValue::from(ns.0.take())
    }
}

impl Queue {
    /// Sends a message to the Queue.
    pub async fn send<T>(&self, message: &T) -> Result<()>
    where
        T: Serialize, {
        let fut = {
            let js_value = serde_wasm_bindgen::to_value(message)?;
            SendJsFuture::from(self.0.send(js_value))
        };

        fut.await.map_err(WorkerError::from_promise_err)?;
        Ok(())
    }
}
