use std::{convert::TryFrom, marker::PhantomData};

use crate::{futures::SendJsFuture, Date, Error, Result};
use js_sys::Array;
use send_wrapper::SendWrapper;
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;
use worker_sys::{MessageBatch as MessageBatchSys, Queue as EdgeQueue};

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

    /// Iterator that deserializes messages in the message batch. Ordering of messages is not guaranteed.
    pub fn iter(&self) -> MessageIter<'_, T>
    where
        T: DeserializeOwned,
    {
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

    /// An array of messages in the batch. Ordering of messages is not guaranteed.
    pub fn messages(&self) -> Result<Vec<Message<T>>>
    where
        T: DeserializeOwned,
    {
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
        let date = js_sys::Date::from(js_sys::Reflect::get(message, self.0.timestamp_key)?);
        let id = js_sys::Reflect::get(message, self.0.id_key)?
            .as_string()
            .ok_or(Error::JsError(
                "Invalid message batch. Failed to get id from message.".to_string(),
            ))?;
        let body =
            serde_wasm_bindgen::from_value(js_sys::Reflect::get(message, self.0.body_key)?)?;

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
        let index = self.0.range.next()?;
        let value = self.0.array.get(index);
        Some(self.parse_message(&value))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.range.size_hint()
    }
}

impl<T> std::iter::DoubleEndedIterator for MessageIter<'_, T>
where
    T: DeserializeOwned,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let index = self.0.range.next_back()?;
        let value = self.0.array.get(index);

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
