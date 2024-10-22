use crate::body::HttpBody;
use bytes::Bytes;
use http_body_util::BodyExt as _;

/// Concatenate the buffers from a body into a single `Bytes` asynchronously.
///
/// This may require copying the data into a single buffer.
///
/// # Note
///
/// Care needs to be taken if the remote is untrusted. The function doesn't
/// implement any length checks and an malicious peer might make it consume
/// arbitrary amounts of memory. Checking the `Content-Length` is a possibility,
/// but it is not strictly mandated to be present.
pub async fn http_body_to_bytes<T>(body: T) -> Result<Bytes, T::Error>
where
    T: HttpBody,
{
    Ok(body.collect().await?.to_bytes())
}
