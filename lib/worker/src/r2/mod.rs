use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

use betterworker_sys::{
    FixedLengthStream as EdgeFixedLengthStream, R2Bucket as EdgeR2Bucket,
    R2MultipartUpload as EdgeR2MultipartUpload, R2Object as EdgeR2Object,
    R2ObjectBody as EdgeR2ObjectBody, R2Objects as EdgeR2Objects,
    R2UploadedPart as EdgeR2UploadedPart,
};
pub use builder::*;
use js_sys::{JsString, Object, Reflect, Uint8Array};
use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, JsValue};

use crate::date::Date;
use crate::error::WorkerError;
use crate::futures::future_from_promise;
use crate::result::Result;
use crate::streams::{ByteStream, FixedLengthStream};

mod builder;

/// An instance of the R2 bucket binding.
pub struct Bucket(SendWrapper<EdgeR2Bucket>);

impl Bucket {
    /// Retrieves the [Object] for the given key containing only object
    /// metadata, if the key exists.
    pub async fn head(&self, key: impl Into<String>) -> Result<Option<R2Object>> {
        let fut = {
            let head_promise = self.0.head(key.into());
            future_from_promise(head_promise)
        };
        let value = fut.await.map_err(WorkerError::from_promise_err)?;

        if value.is_null() {
            return Ok(None);
        }

        Ok(Some(R2Object {
            inner: ObjectInner::NoBody(SendWrapper::new(value.into())),
        }))
    }

    /// Retrieves the [Object] for the given key containing object metadata and
    /// the object body if the key exists. In the event that a precondition
    /// specified in options fails, get() returns an [Object] with no body.
    pub fn get(&self, key: impl Into<String>) -> GetOptionsBuilder {
        GetOptionsBuilder {
            edge_bucket: &self.0,
            key: key.into(),
            only_if: None,
            range: None,
        }
    }

    /// Stores the given `value` and metadata under the associated `key`. Once
    /// the write succeeds, returns an [Object] containing metadata about
    /// the stored Object.
    ///
    /// R2 writes are strongly consistent. Once the future resolves, all
    /// subsequent read operations will see this key value pair globally.
    pub fn put(&self, key: impl Into<String>, value: impl Into<Data>) -> PutOptionsBuilder {
        PutOptionsBuilder {
            edge_bucket: &self.0,
            key: key.into(),
            value: value.into(),
            http_metadata: None,
            custom_metadata: None,
            md5: None,
        }
    }

    /// Deletes the given value and metadata under the associated key. Once the
    /// delete succeeds, returns void.
    ///
    /// R2 deletes are strongly consistent. Once the Promise resolves, all
    /// subsequent read operations will no longer see this key value pair
    /// globally.
    pub async fn delete(&self, key: impl Into<String>) -> Result<()> {
        let fut = {
            let delete_promise = self.0.delete(key.into());
            future_from_promise(delete_promise)
        };

        fut.await.map_err(WorkerError::from_promise_err)?;
        Ok(())
    }

    /// Returns an [Objects] containing a list of [Objects]s contained within
    /// the bucket. By default, returns the first 1000 entries.
    pub fn list(&self) -> ListOptionsBuilder {
        ListOptionsBuilder {
            edge_bucket: &self.0,
            limit: None,
            prefix: None,
            cursor: None,
            delimiter: None,
            include: None,
        }
    }

    /// Creates a multipart upload.
    ///
    /// Returns a [MultipartUpload] value representing the newly created
    /// multipart upload. Once the multipart upload has been created, the
    /// multipart upload can be immediately interacted with globally, either
    /// through the Workers API, or through the S3 API.
    pub fn create_multipart_upload(
        &self, key: impl Into<String>,
    ) -> CreateMultipartUploadOptionsBuilder {
        CreateMultipartUploadOptionsBuilder {
            edge_bucket: &self.0,
            key: key.into(),
            http_metadata: None,
            custom_metadata: None,
        }
    }

    /// Returns an object representing a multipart upload with the given `key`
    /// and `uploadId`.
    ///
    /// The operation does not perform any checks to ensure the validity of the
    /// `uploadId`, nor does it verify the existence of a corresponding
    /// active multipart upload. This is done to minimize latency before
    /// being able to call subsequent operations on the returned object.
    pub fn resume_multipart_upload(
        &self, key: impl Into<String>, upload_id: impl Into<String>,
    ) -> Result<MultipartUpload> {
        Ok(MultipartUpload {
            inner: SendWrapper::new(
                self.0
                    .resume_multipart_upload(key.into(), upload_id.into())
                    .into(),
            ),
        })
    }
}

impl AsRef<JsValue> for Bucket {
    fn as_ref(&self) -> &JsValue {
        &self.0
    }
}

impl TryFrom<Object> for Bucket {
    type Error = WorkerError;

    fn try_from(obj: Object) -> Result<Self> {
        const TYPE_NAME: &'static str = "R2Bucket";

        let data = if obj.constructor().name() == TYPE_NAME {
            obj.unchecked_into()
        } else {
            return Err(WorkerError::InvalidBinding);
        };
        Ok(Self(SendWrapper::new(data)))
    }
}

impl From<Bucket> for JsValue {
    fn from(ns: Bucket) -> Self {
        JsValue::from(ns.0.take())
    }
}

/// [Object] is created when you [put](Bucket::put) an object into a [Bucket].
/// [Object] represents the metadata of an object based on the information
/// provided by the uploader. Every object that you [put](Bucket::put) into a
/// [Bucket] will have an [Object] created.
pub struct R2Object {
    inner: ObjectInner,
}

impl R2Object {
    pub fn key(&self) -> String {
        match &self.inner {
            ObjectInner::NoBody(inner) => inner.key(),
            ObjectInner::Body(inner) => inner.key(),
        }
    }

    pub fn version(&self) -> String {
        match &self.inner {
            ObjectInner::NoBody(inner) => inner.version(),
            ObjectInner::Body(inner) => inner.version(),
        }
    }

    pub fn size(&self) -> u32 {
        match &self.inner {
            ObjectInner::NoBody(inner) => inner.size(),
            ObjectInner::Body(inner) => inner.size(),
        }
    }

    pub fn etag(&self) -> String {
        match &self.inner {
            ObjectInner::NoBody(inner) => inner.etag(),
            ObjectInner::Body(inner) => inner.etag(),
        }
    }

    pub fn http_etag(&self) -> String {
        match &self.inner {
            ObjectInner::NoBody(inner) => inner.http_etag(),
            ObjectInner::Body(inner) => inner.http_etag(),
        }
    }

    pub fn uploaded(&self) -> Date {
        match &self.inner {
            ObjectInner::NoBody(inner) => inner.uploaded(),
            ObjectInner::Body(inner) => inner.uploaded(),
        }
        .into()
    }

    pub fn http_metadata(&self) -> HttpMetadata {
        match &self.inner {
            ObjectInner::NoBody(inner) => inner.http_metadata(),
            ObjectInner::Body(inner) => inner.http_metadata(),
        }
        .into()
    }

    pub fn custom_metadata(&self) -> Result<HashMap<String, String>> {
        let metadata = match &self.inner {
            ObjectInner::NoBody(inner) => inner.custom_metadata(),
            ObjectInner::Body(inner) => inner.custom_metadata(),
        };

        let keys = js_sys::Object::keys(&metadata).to_vec();
        let mut map = HashMap::with_capacity(keys.len());

        for key in keys {
            let key = key.unchecked_into::<JsString>();
            let value = Reflect::get(&metadata, &key)
                .map_err(WorkerError::from_js_err)?
                .dyn_into::<JsString>()
                .map_err(WorkerError::from_cast_err)?;
            map.insert(key.into(), value.into());
        }

        Ok(map)
    }

    pub fn range(&self) -> Result<Range> {
        match &self.inner {
            ObjectInner::NoBody(inner) => inner.range(),
            ObjectInner::Body(inner) => inner.range(),
        }
        .try_into()
    }

    pub fn body(&self) -> Option<ObjectBody> {
        match &self.inner {
            ObjectInner::NoBody(_) => None,
            ObjectInner::Body(body) => Some(ObjectBody { inner: body }),
        }
    }

    pub fn body_used(&self) -> Option<bool> {
        match &self.inner {
            ObjectInner::NoBody(_) => None,
            ObjectInner::Body(inner) => Some(inner.body_used()),
        }
    }

    pub fn write_http_metadata(&self, headers: http::HeaderMap) -> Result<()> {
        let h = web_sys::Headers::new().unwrap();
        for (name, value) in headers
            .iter()
            .filter_map(|(name, value)| value.to_str().map(|value| (name.as_str(), value)).ok())
        {
            h.append(name, value).map_err(WorkerError::from_js_err)?;
        }

        match &self.inner {
            ObjectInner::NoBody(inner) => inner
                .write_http_metadata(h)
                .map_err(WorkerError::from_js_err)?,
            ObjectInner::Body(inner) => inner
                .write_http_metadata(h)
                .map_err(WorkerError::from_js_err)?,
        };

        Ok(())
    }
}

/// The data contained within an [Object].
pub struct ObjectBody<'body> {
    inner: &'body EdgeR2ObjectBody,
}

impl<'body> ObjectBody<'body> {
    /// Reads the data in the [Object] via a [ByteStream].
    pub fn stream(self) -> Result<ByteStream> {
        if self.inner.body_used() {
            return Err(WorkerError::BodyUsed);
        }

        let stream = self.inner.body();
        let stream = wasm_streams::ReadableStream::from_raw(stream.unchecked_into());
        Ok(ByteStream {
            inner: SendWrapper::new(stream.into_stream()),
        })
    }

    pub async fn bytes(self) -> Result<Vec<u8>> {
        let fut = future_from_promise(self.inner.array_buffer());
        let js_buffer = fut.await.map_err(WorkerError::from_promise_err)?;

        let js_buffer = Uint8Array::new(&js_buffer);
        let mut bytes = vec![0; js_buffer.length() as usize];
        js_buffer.copy_to(&mut bytes);

        Ok(bytes)
    }

    pub async fn text(self) -> Result<String> {
        Ok(String::from_utf8(self.bytes().await?)?)
    }
}

/// [UploadedPart] represents a part that has been uploaded.
/// [UploadedPart] objects are returned from
/// [upload_part](MultipartUpload::upload_part) operations and must be passed to
/// the [complete](MultipartUpload::complete) operation.
pub struct UploadedPart {
    inner: SendWrapper<EdgeR2UploadedPart>,
}

impl UploadedPart {
    pub fn part_number(&self) -> u16 {
        self.inner.part_number()
    }

    pub fn etag(&self) -> String {
        self.inner.etag()
    }
}

/// [MultipartUpload] represents an in-progress multipart upload.
/// [MultipartUpload] objects are returned from
/// [create_multipart_upload](Bucket::create_multipart_upload) operations and
/// must be passed to the [complete](MultipartUpload::complete) operation to
/// complete the multipart upload.
pub struct MultipartUpload {
    inner: SendWrapper<EdgeR2MultipartUpload>,
}

impl MultipartUpload {
    /// Uploads a single part with the specified part number to this multipart
    /// upload.
    ///
    /// Returns an [UploadedPart] object containing the etag and part number.
    /// These [UploadedPart] objects are required when completing the multipart
    /// upload.
    ///
    /// Getting hold of a value of this type does not guarantee that there is an
    /// active underlying multipart upload corresponding to that object.
    ///
    /// A multipart upload can be completed or aborted at any time, either
    /// through the S3 API, or by a parallel invocation of your Worker.
    /// Therefore it is important to add the necessary error handling code
    /// around each operation on the [MultipartUpload] object in case the
    /// underlying multipart upload no longer exists.
    pub async fn upload_part(
        &self, part_number: u16, value: impl Into<Data>,
    ) -> Result<UploadedPart> {
        let fut = future_from_promise(self.inner.upload_part(part_number, value.into().into()));
        let uploaded_part = fut.await.map_err(WorkerError::from_promise_err)?;

        Ok(UploadedPart {
            inner: SendWrapper::new(uploaded_part.into()),
        })
    }

    /// Aborts the multipart upload.
    pub async fn abort(&self) -> Result<()> {
        let fut = future_from_promise(self.inner.abort());
        fut.await.map_err(WorkerError::from_promise_err)?;
        Ok(())
    }

    /// Completes the multipart upload with the given parts.
    /// When the future is ready, the object is immediately accessible globally
    /// by any subsequent read operation.
    pub async fn complete(
        self, uploaded_parts: impl IntoIterator<Item = UploadedPart>,
    ) -> Result<R2Object> {
        let fut = future_from_promise(
            self.inner.complete(
                uploaded_parts
                    .into_iter()
                    .map(|part| part.inner.take().into())
                    .collect(),
            ),
        );
        let object = fut.await.map_err(WorkerError::from_promise_err)?;
        Ok(R2Object {
            inner: ObjectInner::Body(SendWrapper::new(object.into())),
        })
    }

    /// Returns the key of the object associated with this multipart upload.
    pub fn key(&self) -> String {
        self.inner.key()
    }

    /// Returns the upload ID of this multipart upload.
    pub fn upload_id(&self) -> String {
        self.inner.upload_id()
    }
}

/// A series of [Object]s returned by [list](Bucket::list).
pub struct R2Objects {
    inner: SendWrapper<EdgeR2Objects>,
}

impl R2Objects {
    /// An [Vec] of [Object] matching the [list](Bucket::list) request.
    pub fn objects(&self) -> Vec<R2Object> {
        self.inner
            .objects()
            .into_iter()
            .map(|raw| R2Object {
                inner: ObjectInner::NoBody(SendWrapper::new(raw)),
            })
            .collect()
    }

    /// If true, indicates there are more results to be retrieved for the
    /// current [list](Bucket::list) request.
    pub fn truncated(&self) -> bool {
        self.inner.truncated()
    }

    /// A token that can be passed to future [list](Bucket::list) calls to
    /// resume listing from that point. Only present if truncated is true.
    pub fn cursor(&self) -> Option<String> {
        self.inner.cursor()
    }

    /// If a delimiter has been specified, contains all prefixes between the
    /// specified prefix and the next occurence of the delimiter.
    ///
    /// For example, if no prefix is provided and the delimiter is '/',
    /// `foo/bar/baz` would return `foo` as a delimited prefix. If `foo/`
    /// was passed as a prefix with the same structure and delimiter,
    /// `foo/bar` would be returned as a delimited prefix.
    pub fn delimited_prefixes(&self) -> Vec<String> {
        self.inner
            .delimited_prefixes()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}

#[derive(Clone)]
pub(crate) enum ObjectInner {
    NoBody(SendWrapper<EdgeR2Object>),
    Body(SendWrapper<EdgeR2ObjectBody>),
}

pub enum Data {
    Stream(FixedLengthStream),
    Text(String),
    Bytes(Vec<u8>),
    Empty,
}

impl From<FixedLengthStream> for Data {
    fn from(stream: FixedLengthStream) -> Self {
        Data::Stream(stream)
    }
}

impl From<String> for Data {
    fn from(value: String) -> Self {
        Data::Text(value)
    }
}

impl From<Vec<u8>> for Data {
    fn from(value: Vec<u8>) -> Self {
        Data::Bytes(value)
    }
}

impl From<Data> for JsValue {
    fn from(data: Data) -> Self {
        match data {
            Data::Stream(stream) => {
                let stream_sys: EdgeFixedLengthStream = stream.into();
                stream_sys.readable().into()
            },
            Data::Text(text) => JsString::from(text).into(),
            Data::Bytes(bytes) => {
                let arr = Uint8Array::new_with_length(bytes.len() as u32);
                arr.copy_from(&bytes);
                arr.into()
            },
            Data::Empty => JsValue::NULL,
        }
    }
}
