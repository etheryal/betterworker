use betterworker::http::Method;
use betterworker::prelude::*;

#[allow(dead_code)]
pub async fn basic_test(env: &Env) -> Result<(), WorkerError> {
    let namespace: ObjectNamespace = env.durable_object("MY_CLASS")?;
    let id = namespace.id_from_name("A")?;
    let bad = env.durable_object("DFSDF_FAKE_BINDING");
    assert!(bad.is_err(), "Invalid binding did not raise error");

    let stub = id.get_stub()?;
    let res = stub
        .fetch_with_str("hello")
        .await?
        .into_body()
        .bytes()
        .await
        .map_err(|_| WorkerError::BadEncoding)?;

    let res2 = stub
        .fetch_with_request(
            Request::builder()
                .method(Method::POST)
                .uri("hello")
                .body("lol".into())
                .unwrap(),
        )
        .await?
        .into_body()
        .bytes()
        .await
        .map_err(|_| WorkerError::BadEncoding)?;

    assert!(res == res2, "Durable object responded wrong to 'hello'");

    let res = stub
        .fetch_with_str("storage")
        .await?
        .into_body()
        .bytes()
        .await
        .map_err(|_| WorkerError::BadEncoding)?;

    let num = std::str::from_utf8(&res)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .expect("Durable Object responded wrong to 'storage'");

    let res = stub
        .fetch_with_str("storage")
        .await?
        .into_body()
        .bytes()
        .await
        .map_err(|_| WorkerError::BadEncoding)?;

    let num2 = std::str::from_utf8(&res)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .expect("Durable Object responded wrong to 'storage'");

    assert!(
        num2 == num + 1,
        "Durable object responded wrong to 'storage'"
    );

    let res = stub
        .fetch_with_str("transaction")
        .await?
        .into_body()
        .bytes()
        .await
        .map_err(|_| WorkerError::BadEncoding)?;

    let num = std::str::from_utf8(&res)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .ok_or_else(|| {
            "Durable Object responded wrong to 'transaction': ".to_string()
                + std::str::from_utf8(&res).unwrap_or("<malformed>")
        })
        .unwrap();

    assert!(
        num == num2 + 1,
        "Durable object responded wrong to 'storage'"
    );

    Ok(())
}
