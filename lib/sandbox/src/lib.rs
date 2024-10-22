use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use betterworker::http::response::{from_web_sys_response, into_web_sys_response};
use betterworker::http::{header, Method, StatusCode};
use betterworker::prelude::*;
use betterworker::{event, wasm_bindgen_futures};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

mod alarm;
mod counter;
mod r2;
mod test;
mod utils;

#[derive(Deserialize, Serialize)]
struct MyData {
    message: String,
    #[serde(default)]
    is: bool,
    #[serde(default)]
    data: Vec<u8>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiData {
    user_id: i32,
    title: String,
    completed: bool,
}

#[derive(Deserialize, Serialize)]
struct FileSize {
    name: String,
    size: u32,
}

fn handle_a_request(req: Request<Body>) -> Response<Body> {
    let cf = req.extensions().get::<Cf>().unwrap();

    Response::new(
        format!(
            "req at: {}, located at: {:?}, within: {}",
            req.uri().path(),
            cf.coordinates().unwrap_or_default(),
            cf.region().unwrap_or_else(|| "unknown region".into())
        )
        .into(),
    )
}

#[derive(Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub age: i32,
}

#[derive(Deserialize, Serialize)]
pub struct CreatePerson {
    pub name: String,
    pub age: i32,
}

static GLOBAL_STATE: AtomicBool = AtomicBool::new(false);

static GLOBAL_QUEUE_STATE: Mutex<Vec<QueueBody>> = Mutex::new(Vec::new());

// We're able to specify a start event that is called when the WASM is
// initialized before any requests. This is useful if you have some global state
// or setup code, like a logger. This is only called once for the entire
// lifetime of the betterworker.
#[event(start)]
pub fn start() {
    utils::set_panic_hook();

    // Change some global state so we know that we ran our setup function.
    GLOBAL_STATE.store(true, Ordering::SeqCst);
}

#[event(fetch)]
pub async fn main(req: Request<Body>, env: Env, _ctx: Context) -> Result<Response<Body>, WorkerError> {
    let res = match (req.method().clone(), req.uri().path()) {
        (Method::GET, "/request") => handle_a_request(req),
        (Method::GET, "/empty") => {
            let res = into_web_sys_response(Response::new("".into()));
            assert!(res.body().is_none());

            let res = from_web_sys_response(res);
            let res = into_web_sys_response(res);
            assert!(res.body().is_none());

            from_web_sys_response(res)
        },
        (Method::GET, "/body") => Response::new("body".into()),
        (Method::GET, "/status-code") => Response::builder()
            .status(StatusCode::IM_A_TEAPOT)
            .body(().into())
            .unwrap(),
        (Method::POST, "/bytes") => {
            let bytes = req.into_body().bytes().await.unwrap();
            assert_eq!(bytes, [0u8, 1, 2][..]);
            Response::new(bytes.into())
        },
        (Method::POST, "/headers") => {
            let mut headers = req.headers().clone();
            headers.append("Hello", "World!".parse().unwrap());

            let mut res = Response::new("returned your headers to you.".into());
            *res.headers_mut() = headers;
            res
        },
        (Method::POST, "/echo") => Response::new(req.into_body()),
        (Method::GET, "/async-text-echo") => Response::new(req.into_body()),
        (Method::GET, "/fetch") => {
            let req = Request::post("https://example.com").body(()).unwrap();
            let resp = fetch(req).await?;

            Response::new(format!("received response with status code {:?}", resp.status()).into())
        },
        (Method::GET, "/fetch-cancelled") => {
            let controller = AbortController::default();
            let signal = controller.signal();

            let (tx, rx) = futures_channel::oneshot::channel();

            // Spawns a future that'll make our fetch request and not block this function.
            wasm_bindgen_futures::spawn_local(async move {
                let res = fetch(
                    Request::get("https://cloudflare.com")
                        .extension(signal)
                        .body(())
                        .unwrap(),
                )
                .await;

                tx.send(res).unwrap();
            });

            // And then we try to abort that fetch as soon as we start it, hopefully before
            // cloudflare.com responds.
            controller.abort();

            let res = rx.await.unwrap();
            res.unwrap_or_else(|err| Response::new(err.to_string().into()))
        },
        (Method::GET, "/secret") => Response::new(env.secret("SOME_SECRET")?.to_string().into()),
        (Method::GET, "/wait-1s") => {
            Delay::from(Duration::from_secs(1)).await;
            Response::new(().into())
        },
        (Method::GET, "/init-called") => {
            let init_called = GLOBAL_STATE.load(Ordering::SeqCst);
            Response::new(init_called.to_string().into())
        },
        (Method::GET, "/cache") => {
            let cache = Cache::default();
            if let Some(resp) = cache.get(req.uri().to_string(), true).await? {
                resp
            } else {
                Response::new("cache miss".into())
            }
        },
        (Method::PUT, "/cache") => {
            let cache = Cache::default();

            let resp = Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::CACHE_CONTROL, "s-maxage=10")
                .body(
                    serde_json::to_string(
                        &serde_json::json!({ "timestamp": Date::now().as_millis() }),
                    )
                    .unwrap()
                    .into(),
                )
                .unwrap();

            cache.put(req.uri().to_string(), resp).await?;
            cache
                .get(req.uri().to_string(), false)
                .await
                .unwrap()
                .unwrap()
        },
        (Method::DELETE, "/cache") => {
            let cache = Cache::default();

            let res = cache.delete(req.uri().to_string(), true).await?;
            Response::new(serde_json::to_string(&res)?.into())
        },
        (Method::GET, "/kv") => {
            let kv = env.kv("SOME_NAMESPACE")?;
            kv.put("foo", "bar")?.execute().await?;

            Response::new(
                serde_json::to_string(&kv.list().execute().await?)
                    .unwrap()
                    .into(),
            )
        },
        (Method::GET, "/durable") => {
            let namespace = env.durable_object("COUNTER")?;
            let stub = namespace.id_from_name("A")?.get_stub()?;
            // when calling fetch to a Durable Object, a full URL must be used.
            // Alternatively, a compatibility flag can be provided in
            // wrangler.toml to opt-in to older behavior: https://developers.cloudflare.com/workers/platform/compatibility-dates#durable-object-stubfetch-requires-a-full-url
            stub.fetch_with_str("https://fake-host/").await?
        },
        (Method::GET, "/durable/alarm") => {
            let namespace = env.durable_object("ALARM")?;
            let stub = namespace.id_from_name("alarm")?.get_stub()?;
            // when calling fetch to a Durable Object, a full URL must be used.
            // Alternatively, a compatibility flag can be provided in
            // wrangler.toml to opt-in to older behavior: https://developers.cloudflare.com/workers/platform/compatibility-dates#durable-object-stubfetch-requires-a-full-url
            stub.fetch_with_str("https://fake-host/alarm").await?
        },
        (Method::GET, "/service-binding") => {
            let fetcher = env.service("remote")?;
            fetcher.fetch(req).await?
        },
        (Method::POST, "/queue/send/12345") => {
            let queue = env.queue("my_queue")?;
            queue
                .send(&QueueBody {
                    id: "12345".to_string(),
                })
                .await?;

            Response::new("Message sent".into())
        },
        (Method::GET, "/queue") => {
            let guard = GLOBAL_QUEUE_STATE.lock().unwrap();
            let messages: Vec<QueueBody> = guard.clone();
            Response::new(serde_json::to_string(&messages).unwrap().into())
        },
        (Method::GET, "/r2/list-empty") => r2::list_empty(&env).await?,
        (Method::GET, "/r2/list") => r2::list(&env).await?,
        (Method::GET, "/r2/get-empty") => r2::get_empty(&env).await?,
        (Method::GET, "/r2/get") => r2::get(&env).await?,
        (Method::PUT, "/r2/put") => r2::put(&env).await?,
        (Method::PUT, "/r2/put-properties") => r2::put_properties(&env).await?,
        (Method::PUT, "/r2/put-multipart") => r2::put_multipart(&env).await?,
        (Method::DELETE, "/r2/delete") => r2::delete(&env).await?,
        (Method::GET, "/websocket") => {
            // Accept / handle a websocket connection
            let pair = WebSocketPair::new()?;
            let server = pair.server;
            server.accept()?;

            let some_namespace_kv = env.kv("SOME_NAMESPACE")?;

            wasm_bindgen_futures::spawn_local(async move {
                let mut event_stream = server.events().expect("could not open stream");

                while let Some(event) = event_stream.next().await {
                    match event.expect("received error in websocket") {
                        WebsocketEvent::Message(msg) => {
                            if let Some(text) = msg.text() {
                                server.send_with_str(text).expect("could not relay text");
                            }
                        },
                        WebsocketEvent::Close(_) => {
                            // Sets a key in a test KV so the integration tests can query if we
                            // actually got the close event. We can't use the shared dat a for this
                            // because miniflare resets that every request.
                            some_namespace_kv
                                .put("got-close-event", "true")
                                .unwrap()
                                .execute()
                                .await
                                .unwrap();
                        },
                    }
                }
            });

            Response::builder()
                .status(StatusCode::SWITCHING_PROTOCOLS)
                .extension(pair.client)
                .body(().into())
                .unwrap()
        },
        (Method::GET, "/websocket/closed") => {
            let some_namespace_kv = env.kv("SOME_NAMESPACE")?;
            let got_close_event = some_namespace_kv
                .get("got-close-event")
                .text()
                .await?
                .unwrap_or_else(|| "false".into());

            // Let the integration tests have some way of knowing if we successfully
            // received the closed event.
            Response::new(got_close_event.into())
        },
        (Method::POST, "/d1/exec") => {
            let d1 = env.d1("DB")?;
            let query = req.into_body().text().await?;
            let exec_result = d1.exec(&query).await;
            match exec_result {
                Ok(result) => {
                    let count = result.count().unwrap_or_default();
                    Response::new(format!("{}", count).into())
                },
                Err(err) => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(format!("Exec failed - {}", err).into())
                    .unwrap(),
            }
        },
        (Method::GET, "/d1/people") => {
            let d1: Database = env.d1("DB")?;
            let people = d1.prepare("select * from people;").all::<Person>().await?;
            Response::new(serde_json::to_string(people.results()).unwrap().into())
        },
        (Method::POST, "/d1/people") => {
            let create_person: CreatePerson =
                serde_json::from_str(req.into_body().text().await?.as_str()).unwrap();
            let d1: Database = env.d1("DB")?;
            let new_person = d1
                .prepare("insert into people (name, age) values(?, ?) returning *;")
                .bind_many(&[
                    create_person.name.to_string().into(),
                    create_person.age.into(),
                ])
                .unwrap()
                .first::<Person>(None)
                .await?
                .unwrap();
            Response::new(serde_json::to_string(&new_person).unwrap().into())
        },
        _ => panic!("unknown uri {}", req.uri()),
    };

    Ok(res)
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct QueueBody {
    pub id: String,
}

#[event(queue)]
pub async fn queue(
    message_batch: MessageBatch<QueueBody>, _env: Env, _ctx: Context,
) -> Result<(), WorkerError> {
    let mut guard = GLOBAL_QUEUE_STATE.lock().unwrap();
    for message in message_batch.messages()? {
        console_log!(
            "Received queue message {:?}, with id {} and timestamp: {}",
            message.body,
            message.id,
            message.timestamp.to_string()
        );
        guard.push(message.body);
    }
    Ok(())
}
