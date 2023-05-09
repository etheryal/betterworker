use std::{marker::PhantomData, convert::TryFrom};

use crate::{futures::SendJsFuture, Date, Error, Result};
use js_sys::Array;
use send_wrapper::SendWrapper;
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;
use worker_sys::{MessageBatch as MessageBatchSys, Queue as EdgeQueue};

static BODY_KEY_STR: &str = "body";
static ID_KEY_STR: &str = "id";
static TIMESTAMP_KEY_STR: &str = "timestamp";

pub struct MessageBatch<T> {
    inner: MessageBatchSys,
    messages: Array,
    data: PhantomData<T>,
    timestamp_key: JsValue,
    body_key: JsValue,
    id_key: JsValue,
}

unsafe impl<T> Send for MessageBatch<T> {}
unsafe impl<T> Sync for MessageBatch<T> {}

impl<T> MessageBatch<T> {
    pub fn new(message_batch_sys: MessageBatchSys) -> Self {
        let timestamp_key = JsValue::from_str(TIMESTAMP_KEY_STR);
        let body_key = JsValue::from_str(BODY_KEY_STR);
        let id_key = JsValue::from_str(ID_KEY_STR);
        Self {
            messages: message_batch_sys.messages(),
            inner: message_batch_sys,
            data: PhantomData,
            timestamp_key,
            body_key,
            id_key,
        }
    }
}

pub struct Message<T> {
    pub body: T,
    pub timestamp: Date,
    pub id: String,
}

unsafe impl<T> Send for Message<T> {}
unsafe impl<T> Sync for Message<T> {}

impl<T> MessageBatch<T> {
    /// The name of the Queue that belongs to this batch.
    pub fn queue(&self) -> String {
        self.inner.queue()
    }

    /// Marks every message to be retried in the next batch.
    pub fn retry_all(&self) {
        self.inner.retry_all();
    }

    /// Iterator that deserializes messages in the message batch. Ordering of messages is not guaranteed.
    pub fn iter(&self) -> MessageIter<'_, T>
    where
        T: DeserializeOwned,
    {
        MessageIter {
            range: 0..self.messages.length(),
            array: &self.messages,
            timestamp_key: &self.timestamp_key,
            body_key: &self.body_key,
            id_key: &self.id_key,
            data: PhantomData,
        }
    }

    /// An array of messages in the batch. Ordering of messages is not guaranteed.
    pub fn messages(&self) -> Result<Vec<Message<T>>>
    where
        T: DeserializeOwned,
    {
        self.iter().collect()
    }
}

pub struct MessageIter<'a, T> {
    range: std::ops::Range<u32>,
    array: &'a Array,
    timestamp_key: &'a JsValue,
    body_key: &'a JsValue,
    id_key: &'a JsValue,
    data: PhantomData<T>,
}

unsafe impl<T> Send for MessageIter<'_, T> {}
unsafe impl<T> Sync for MessageIter<'_, T> {}

impl<T> MessageIter<'_, T>
where
    T: DeserializeOwned,
{
    fn parse_message(&self, message: &JsValue) -> Result<Message<T>> {
        let date = js_sys::Date::from(js_sys::Reflect::get(message, self.timestamp_key)?);
        let id = js_sys::Reflect::get(message, self.id_key)?
            .as_string()
            .ok_or(Error::JsError(
                "Invalid message batch. Failed to get id from message.".to_string(),
            ))?;
        let body = serde_wasm_bindgen::from_value(js_sys::Reflect::get(message, self.body_key)?)?;

        Ok(Message {
            id,
            body,
            timestamp: Date::from(date),
        })
    }
}

impl<T> std::iter::Iterator for MessageIter<'_, T>
where
    T: DeserializeOwned,
{
    type Item = Result<Message<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.range.next()?;

        let value = self.array.get(index);

        Some(self.parse_message(&value))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<T> std::iter::DoubleEndedIterator for MessageIter<'_, T>
where
    T: DeserializeOwned,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let index = self.range.next_back()?;
        let value = self.array.get(index);

        Some(self.parse_message(&value))
    }
}

impl<'a, T> std::iter::FusedIterator for MessageIter<'a, T> where T: DeserializeOwned {}

impl<'a, T> std::iter::ExactSizeIterator for MessageIter<'a, T> where T: DeserializeOwned {}

pub struct Queue(SendWrapper<EdgeQueue>);

impl AsRef<JsValue> for Queue {
    fn as_ref(&self) -> &JsValue {
        self.0.as_ref()
    }
}

impl TryFrom<JsValue> for Queue {
    type Error = crate::Error;

    fn try_from(val: JsValue) -> Result<Self> {
        Ok(Self(SendWrapper::new(val.dyn_into()?)))
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
        T: Serialize,
    {
        let fut = {
            let js_value = serde_wasm_bindgen::to_value(message)?;
            SendJsFuture::from(self.0.send(js_value))
        };

        fut.await.map_err(Error::from)?;
        Ok(())
    }
}
