use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parse_macro_input, Ident, ItemFn};

pub fn expand_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs: Punctuated<Ident, Comma> =
        parse_macro_input!(attr with Punctuated::parse_terminated);

    enum HandlerType {
        Fetch,
        Scheduled,
        Start,
        #[cfg(feature = "queue")]
        Queue,
    }
    use HandlerType::*;

    let mut handler_type = None;
    let mut respond_with_errors = false;

    for attr in attrs {
        match attr.to_string().as_str() {
            "fetch" => handler_type = Some(Fetch),
            "scheduled" => handler_type = Some(Scheduled),
            "start" => handler_type = Some(Start),
            #[cfg(feature = "queue")]
            "queue" => handler_type = Some(Queue),
            "respond_with_errors" => {
                respond_with_errors = true;
            },
            _ => panic!("Invalid attribute: {}", attr),
        }
    }
    let handler_type = handler_type.expect(
        "must have either 'fetch', 'scheduled', 'queue' or 'start' attribute, e.g. #[event(fetch)]",
    );

    // create new var using syn item of the attributed fn
    let mut input_fn = parse_macro_input!(item as ItemFn);

    match handler_type {
        Fetch => {
            // TODO: validate the inputs / signature
            // save original fn name for re-use in the wrapper fn
            let input_fn_ident = Ident::new(
                &(input_fn.sig.ident.to_string() + "_fetch_glue"),
                input_fn.sig.ident.span(),
            );
            let wrapper_fn_ident = Ident::new("fetch", input_fn.sig.ident.span());
            // rename the original attributed fn
            input_fn.sig.ident = input_fn_ident.clone();

            let error_handling = match respond_with_errors {
                true => {
                    quote! {
                        let res = ::betterworker::http::Response::builder().status(500).body(e.to_string()).unwrap();
                        ::betterworker::http::response::into_web_sys_response(res)
                    }
                },
                false => {
                    quote! { panic!("{}", e) }
                },
            };

            // create a new "main" function that takes the betterworker_sys::Request, and
            // calls the original attributed function, passing in a
            // http::Request
            let wrapper_fn = quote! {
                pub async fn #wrapper_fn_ident(
                    req: ::betterworker::betterworker_sys::web_sys::Request,
                    env: ::betterworker::betterworker_sys::Env,
                    ctx: ::betterworker::betterworker_sys::Context
                ) -> ::betterworker::betterworker_sys::web_sys::Response {
                    let ctx = ::betterworker::context::Context::new(ctx);
                    // get the betterworker::Result<worker::Response> by calling the original fn
                    match #input_fn_ident(::betterworker::http::request::from_web_sys_request(req), ::betterworker::env::Env::from(env), ctx).await.map(::betterworker::http::response::into_web_sys_response) {
                        Ok(res) => res,
                        Err(e) => {
                            ::betterworker::betterworker_sys::console_error!("{}", &e);
                            #error_handling
                        }
                    }
                }
            };
            let wasm_bindgen_code =
                wasm_bindgen_macro_support::expand(TokenStream::new().into(), wrapper_fn)
                    .expect("wasm_bindgen macro failed to expand");

            let output = quote! {
                #input_fn

                mod _worker_fetch {
                    use ::betterworker::{wasm_bindgen, wasm_bindgen_futures};
                    use super::#input_fn_ident;
                    #wasm_bindgen_code
                }
            };

            TokenStream::from(output)
        },
        Scheduled => {
            // save original fn name for re-use in the wrapper fn
            let input_fn_ident = Ident::new(
                &(input_fn.sig.ident.to_string() + "_scheduled_glue"),
                input_fn.sig.ident.span(),
            );
            let wrapper_fn_ident = Ident::new("scheduled", input_fn.sig.ident.span());
            // rename the original attributed fn
            input_fn.sig.ident = input_fn_ident.clone();

            let wrapper_fn = quote! {
                pub async fn #wrapper_fn_ident(event: ::betterworker::betterworker_sys::ScheduledEvent, env: ::betterworker::betterworker_sys::Env, ctx: ::betterworker::betterworker_sys::ScheduleContext) {
                    // call the original fn
                    #input_fn_ident(::betterworker::ScheduledEvent::from(event), ::betterworker::env::Env::from(env), ::betterworker::ScheduleContext::from(ctx)).await
                }
            };
            let wasm_bindgen_code =
                wasm_bindgen_macro_support::expand(TokenStream::new().into(), wrapper_fn)
                    .expect("wasm_bindgen macro failed to expand");

            let output = quote! {
                #input_fn

                mod _worker_scheduled {
                    use ::betterworker::{wasm_bindgen, wasm_bindgen_futures};
                    use super::#input_fn_ident;
                    #wasm_bindgen_code
                }
            };

            TokenStream::from(output)
        },
        #[cfg(feature = "queue")]
        Queue => {
            // save original fn name for re-use in the wrapper fn
            let input_fn_ident = Ident::new(
                &(input_fn.sig.ident.to_string() + "_queue_glue"),
                input_fn.sig.ident.span(),
            );
            let wrapper_fn_ident = Ident::new("queue", input_fn.sig.ident.span());
            // rename the original attributed fn
            input_fn.sig.ident = input_fn_ident.clone();

            let wrapper_fn = quote! {
                pub async fn #wrapper_fn_ident(event: ::betterworker::betterworker_sys::MessageBatch, env: ::betterworker::betterworker_sys::Env, ctx: ::betterworker::betterworker_sys::Context) {
                    // call the original fn
                    let ctx = ::betterworker::context::Context::new(ctx);
                    match #input_fn_ident(::betterworker::queue::MessageBatch::new(event), ::betterworker::env::Env::from(env), ctx).await {
                        Ok(()) => {},
                        Err(e) => {
                            ::betterworker::betterworker_sys::console_error!("{}", &e);
                            panic!("{}", e);
                        }
                    }
                }
            };
            let wasm_bindgen_code =
                wasm_bindgen_macro_support::expand(TokenStream::new().into(), wrapper_fn)
                    .expect("wasm_bindgen macro failed to expand");

            let output = quote! {
                #input_fn

                mod _worker_queue {
                    use ::betterworker::{wasm_bindgen, wasm_bindgen_futures};
                    use super::#input_fn_ident;
                    #wasm_bindgen_code
                }
            };

            TokenStream::from(output)
        },
        Start => {
            // save original fn name for re-use in the wrapper fn
            let input_fn_ident = Ident::new(
                &(input_fn.sig.ident.to_string() + "_start_glue"),
                input_fn.sig.ident.span(),
            );
            let wrapper_fn_ident = Ident::new("start", input_fn.sig.ident.span());
            // rename the original attributed fn
            input_fn.sig.ident = input_fn_ident.clone();

            let wrapper_fn = quote! {
                pub fn #wrapper_fn_ident() {
                    // call the original fn
                    #input_fn_ident()
                }
            };
            let wasm_bindgen_code =
                wasm_bindgen_macro_support::expand(quote! { start }, wrapper_fn)
                    .expect("wasm_bindgen macro failed to expand");

            let output = quote! {
                #input_fn

                mod _worker_start {
                    use ::betterworker::{wasm_bindgen, wasm_bindgen_futures};
                    use super::#input_fn_ident;
                    #wasm_bindgen_code
                }
            };

            TokenStream::from(output)
        },
    }
}
