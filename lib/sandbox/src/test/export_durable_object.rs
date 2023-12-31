use std::collections::HashMap;

use betterworker::http::StatusCode;
use betterworker::prelude::*;
use betterworker::wasm_bindgen;
use serde::Serialize;

#[durable_object]
pub struct MyClass {
    state: State,
    number: usize,
}

#[durable_object]
impl DurableObject for MyClass {
    fn new(state: State, _env: Env) -> Self {
        Self { state, number: 0 }
    }

    async fn fetch(&mut self, req: Request<Body>) -> Result<Response<Body>, WorkerError> {
        let handler = async move {
            match req.uri().path() {
                "/hello" => Ok::<_, WorkerError>(Response::new("Hello!".into())),
                "/storage" => {
                    let mut storage = self.state.storage();
                    let map = [("one".to_string(), 1), ("two".to_string(), 2)]
                        .iter()
                        .cloned()
                        .collect::<HashMap<_, _>>();
                    storage.put("map", map.clone()).await?;
                    storage.put("array", [("one", 1), ("two", 2)]).await?;
                    storage.put("anything", Some(45)).await?;

                    let list = storage.list().await?;
                    let mut keys = vec![];

                    for key in list.keys() {
                        let key = key.unwrap().as_string().expect("Key wasn't a string");
                        keys.push(key);
                    }

                    assert!(
                        keys == vec!["anything", "array", "map"],
                        "Didn't list all of the keys: {keys:?}"
                    );
                    let vals = storage
                        .get_multiple(keys)
                        .await
                        .map_err(|e| e.to_string() + " -- get_multiple")
                        .unwrap();
                    assert!(
                        serde_wasm_bindgen::from_value::<Option<i32>>(
                            vals.get(&"anything".into())
                        )? == Some(45),
                        "Didn't get the right Option<i32> using get_multiple"
                    );
                    assert!(
                        serde_wasm_bindgen::from_value::<[(String, i32); 2]>(
                            vals.get(&"array".into())
                        )? == [("one".to_string(), 1), ("two".to_string(), 2)],
                        "Didn't get the right array using get_multiple"
                    );
                    assert!(
                        serde_wasm_bindgen::from_value::<HashMap<String, i32>>(
                            vals.get(&"map".into())
                        )? == map,
                        "Didn't get the right HashMap<String, i32> using get_multiple"
                    );

                    #[derive(Serialize)]
                    struct Stuff {
                        thing: String,
                        other: i32,
                    }
                    storage
                        .put_multiple(Stuff {
                            thing: "Hello there".to_string(),
                            other: 56,
                        })
                        .await?;

                    assert!(
                        storage.get::<String>("thing").await? == "Hello there",
                        "Didn't put the right thing with put_multiple"
                    );
                    assert!(
                        storage.get::<i32>("other").await? == 56,
                        "Didn't put the right thing with put_multiple"
                    );

                    storage.delete_multiple(vec!["thing", "other"]).await?;

                    self.number = storage.get("count").await.unwrap_or(0) + 1;

                    storage.delete_all().await?;

                    storage.put("count", self.number).await?;
                    Ok(Response::new(self.number.to_string().into()))
                },
                "/transaction" => Ok(Response::builder()
                    .status(StatusCode::NOT_IMPLEMENTED)
                    .body("transactional storage API is still unstable".into())
                    .unwrap()),
                _ => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body("Not Found".into())
                    .unwrap()),
            }
        };
        handler.await.or_else(|err| {
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(err.to_string().into())
                .unwrap())
        })
    }
}
