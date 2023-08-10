# BetterWorkers 🦀

A fork of [workers-rs](https://github.com/cloudflare/workers-rs) that provides a safer and more ergonomic way to write Cloudflare Workers in Rust. It supports the following features:

- [x] [`http`](https://crates.io/crates/http) crate for HTTP types, instead of using the custom types of `workers-rs`. This allows you to use Axum or any other HTTP framework.
- [x] Removed all `unsafe` code, and replaced it with safe wrappers.
- [x] All types are `Send` and `Sync`, so you can use them in async functions, unlike types in `workers-rs`.
- [x] `async`/`.await` support for all event types.

## Example Usage

```rust
use betterworker::prelude::*;

#[event(fetch)]
pub async fn main(req: Request<Body>, _env: Env, _ctx: Context) -> Result<Response<Body>, Error> {
    let cf = req.extensions().get::<Cf>().unwrap();
    console_log!(
        "{} {}, located at: {:?}, within: {}",
        req.method().to_string(),
        req.uri().path(),
        cf.coordinates().unwrap_or_default(),
        cf.region().unwrap_or("unknown region".into())
    );

    Ok(Response::new(Body::from("Hello, world!")))
}
```

## Getting Started

The project uses [wrangler](https://github.com/cloudflare/wrangler2) version 2.x for running and publishing your Worker.

Get the Rust worker project [template](https://github.com/cloudflare/workers-sdk/tree/main/templates/experimental/worker-rust) manually, or run the following command:
```bash
npm init cloudflare project_name worker-rust
cd project_name
```

You should see a new project layout with a `src/lib.rs`. Start there! Use any local or remote crates
and modules (as long as they compile to the `wasm32-unknown-unknown` target).

Once you're ready to run your project:

First check that the wrangler version is 2.x
```bash
npx wrangler --version
```

Then, run your worker

```bash
npx wrangler dev
```

Finally, go live:

```bash
# configure your routes, zones & more in your worker's `wrangler.toml` file
npx wrangler publish
```

If you would like to have `wrangler` installed on your machine, see instructions in [wrangler repository](https://github.com/cloudflare/wrangler2).
## Durable Object, KV, Secret, & Variable Bindings

All "bindings" to your script (Durable Object & KV Namespaces, Secrets, and Variables) are
accessible from the `env` parameter provided to the entrypoint (`main` in this example).

```rust
use betterworker::{
    http::{Method, StatusCode},
    prelude::*,
};

#[event(fetch)]
pub async fn main(req: Request<Body>, env: Env, _ctx: Context) -> Result<Response<Body>, Error> {
    match (req.method().clone(), req.uri().path()) {
        (Method::GET, "/durable") => {
            let namespace = env.durable_object("CHATROOM")?;
            let stub = namespace.id_from_name("A")?.get_stub()?;
            stub.fetch_with_str("/messages").await
        }
        (Method::GET, "/secret") => Ok(Response::new(Body::from(
            env.secret("CF_API_TOKEN")?.to_string(),
        ))),
        (Method::GET, "/var") => Ok(Response::new(Body::from(
            env.var("BUILD_NUMBER")?.to_string(),
        ))),
        (Method::POST, "/kv") => {
            let kv = env.kv("SOME_NAMESPACE")?;
            kv.put("key", "value")?.execute().await?;
            Ok(Response::new(Body::empty()))
        }
        (_, _) => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
    }
}
```

For more information about how to configure these bindings, see:

- https://developers.cloudflare.com/workers/cli-wrangler/configuration#keys
- https://developers.cloudflare.com/workers/learning/using-durable-objects#configuring-durable-object-bindings

## Durable Objects

### Define a Durable Object in Rust

To define a Durable Object using the `worker` crate you need to implement the `DurableObject` trait
on your own struct. Additionally, the `#[durable_object]` attribute macro must be applied to _both_
your struct definition and the trait `impl` block for it.

```rust
#![feature(async_fn_in_trait)]
use betterworker::prelude::*;
use betterworker::wasm_bindgen;

#[durable_object]
pub struct Chatroom {
    users: Vec<User>,
    messages: Vec<Message>,
    state: State,
    env: Env, // access `Env` across requests, use inside `fetch`
}

#[durable_object]
impl DurableObject for Chatroom {
    fn new(state: State, env: Env) -> Self {
        Self {
            users: vec![],
            messages: vec![],
            state: state,
            env,
        }
    }

    async fn fetch(&mut self, _req: Request<Body>) -> Result<Response<Body>, Error> {
        // do some work when a worker makes a request to this DO
        Response::ok(&format!("{} active users.", self.users.len()))
    }
}
```

You'll need to "migrate" your worker script when it's published so that it is aware of this new
Durable Object, and include a binding in your `wrangler.toml`.

- Include the Durable Object binding type in you `wrangler.toml` file:

```toml
# ...

[durable_objects]
bindings = [
  { name = "CHATROOM", class_name = "Chatroom" } # the `class_name` uses the Rust struct identifier name
]

[[migrations]]
tag = "v1" # Should be unique for each entry
new_classes = ["Chatroom"] # Array of new classes
```

- For more information about migrating your Durable Object as it changes, see the docs here:
  https://developers.cloudflare.com/workers/learning/using-durable-objects#durable-object-migrations-in-wranglertoml

## Queues

### Enabling queues
As queues are in beta you need to enable the `queue` feature flag.

Enable it by adding it to the worker dependency in your `Cargo.toml`: 
```toml
worker = {version = "...", features = ["queue"]}
```

### Example worker consuming and producing messages:
```rust
use betterworker::prelude::*;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct MyType {
    foo: String,
    bar: u32,
}

// Consume messages from a queue
#[event(queue)]
pub async fn main(message_batch: MessageBatch<MyType>, env: Env, _ctx: Context) -> Result<(), Error> {
    // Get a queue with the binding 'my_queue'
    let my_queue = env.queue("my_queue")?;

    // Deserialize the message batch
    let messages = message_batch.messages()?;

    // Loop through the messages
    for message in messages {
        // Log the message and meta data
        console_log!(
            "Got message {:?}, with id {} and timestamp: {}",
            message.body,
            message.id,
            message.timestamp.to_string()
        );

        // Send the message body to the other queue
        my_queue.send(&message.body).await?;
    }

    // Retry all messages
    message_batch.retry_all();
    Ok(())
}
```

## D1 Databases

### Enabling D1 databases
As D1 databases are in alpha, you'll need to enable the `d1` feature on the `worker` crate.

```toml
worker = { version = "x.y.z", features = ["d1"] }
```

### Example usage
```rust
use betterworker::{prelude::*, http::Method};
use serde::Deserialize;

#[derive(Deserialize)]
struct Thing {
	thing_id: String,
	desc: String,
	num: u32,
}

#[event(fetch)]
pub async fn main(req: Request<Body>,	env: Env, _ctx: Context) -> Result<Response<Body>, Error> {
    match (req.method().clone(), req.uri().path()) {
        (Method::GET, route) => {
			let d1 = env.d1("things-db")?;
			let statement = d1.prepare("SELECT * FROM things WHERE thing_id = ?1");
			let query = statement.bind(route)?;
			let result = query.first::<Thing>(None).await?;
            let serialized = serde_json::to_string(&result)?;
			match result {
				Some(thing) => Ok(Response::new(Body::new(serialized))),
				None => Ok(Response::builder()
                    .status(404)
                    .body(Body::empty())
                    .unwrap()),
			}
        }
        _ => Ok(Response::builder()
            .status(404)
            .body(Body::empty())
            .unwrap()),
    }
}
```
