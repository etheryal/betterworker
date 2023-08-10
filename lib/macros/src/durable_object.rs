use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{Error, FnArg, ImplItem, Item, Type, TypePath};

pub fn expand_macro(tokens: TokenStream) -> syn::Result<TokenStream> {
    let item = syn::parse2::<Item>(tokens)?;
    match item {
        Item::Impl(imp) => {
            let impl_token = imp.impl_token;
            let trai = imp.trait_.clone();
            let (_, trai, _) = trai.ok_or_else(|| {
                Error::new_spanned(impl_token, "Must be a DurableObject trait impl")
            })?;

            if !trai
                .segments
                .last()
                .map(|x| x.ident == "DurableObject")
                .unwrap_or(false)
            {
                return Err(Error::new(
                    trai.span(),
                    "Must be a DurableObject trait impl",
                ));
            }

            let pound = syn::Token![#](imp.span()).to_token_stream();
            let wasm_bindgen_attr = quote! {
                #pound[::betterworker::wasm_bindgen::prelude::wasm_bindgen]
            };

            let struct_name = imp.self_ty;
            let items = imp.items;
            let mut tokenized = vec![];
            let mut has_alarm = false;

            for item in items {
                let impl_method = match item {
                    ImplItem::Fn(m) => m,
                    _ => {
                        return Err(Error::new_spanned(
                            item,
                            "Impl block must only contain methods",
                        ))
                    },
                };

                let tokens = match impl_method.sig.ident.to_string().as_str() {
                    "new" => {
                        let mut method = impl_method.clone();
                        method.sig.ident = Ident::new("_new", method.sig.ident.span());

                        // modify the `state` argument so it is type ObjectState
                        let state_arg = method
                            .sig
                            .inputs
                            .first_mut()
                            .expect(
                                "DurableObject `new` method must have 2 arguments: state and env",
                            )
                            .into_token_stream();
                        let env_arg = method
                            .sig
                            .inputs
                            .pop()
                            .expect("DurableObject `new` method expects a second argument: env")
                            .into_token_stream();

                        let FnArg::Typed(mut state_pat) = syn::parse2::<FnArg>(state_arg)? else {
                            return Err(Error::new(
                                method.sig.inputs.span(),
                                "DurableObject `new` method expects `state: State` as first \
                                 argument.",
                            ));
                        };
                        let FnArg::Typed(mut env_pat) = syn::parse2::<FnArg>(env_arg)? else {
                            return Err(Error::new(
                                method.sig.inputs.span(),
                                "DurableObject `new` method expects `env: Env` as second argument.",
                            ));
                        };

                        let path = syn::parse2::<TypePath>(quote! {
                            betterworker::betterworker_sys::DurableObjectState
                        })?;
                        state_pat.ty = Box::new(Type::Path(path));

                        let path = syn::parse2::<TypePath>(quote! {
                            betterworker::betterworker_sys::Env
                        })?;
                        env_pat.ty = Box::new(Type::Path(path));

                        let state_name = state_pat.pat.clone();
                        let env_name = env_pat.pat.clone();

                        method.sig.inputs.clear();
                        method.sig.inputs.insert(0, FnArg::Typed(state_pat));
                        method.sig.inputs.insert(1, FnArg::Typed(env_pat));

                        // prepend the function block's statements to convert the ObjectState to
                        // State type
                        let mut prepended = Vec::with_capacity(8);
                        if state_name.to_token_stream().to_string() != "_" {
                            prepended.push(syn::parse_quote! {
                                let state = ::betterworker::durable::State::from(#state_name);
                            });
                        };
                        if env_name.to_token_stream().to_string() != "_" {
                            prepended.push(syn::parse_quote! {
                                let env = ::betterworker::env::Env::from(#env_name);
                            });
                        };
                        prepended.extend(method.block.stmts);
                        method.block.stmts = prepended;

                        quote! {
                            #pound[wasm_bindgen::prelude::wasm_bindgen(constructor)]
                            pub #method
                        }
                    },
                    "fetch" => {
                        let mut method = impl_method.clone();
                        method.sig.ident = Ident::new("_fetch_raw", method.sig.ident.span());
                        quote! {
                            #pound[wasm_bindgen::prelude::wasm_bindgen(js_name = fetch)]
                            pub fn _fetch(&mut self, req: ::betterworker::betterworker_sys::web_sys::Request) -> ::betterworker::js_sys::Promise {
                                // SAFETY:
                                // On the surface, this is unsound because the Durable Object could be dropped
                                // while JavaScript still has possession of the future. However,
                                // we know something that Rust doesn't: that the Durable Object will never be destroyed
                                // while there is still a running promise inside of it, therefore we can let a reference
                                // to the durable object escape into a static-lifetime future.
                                let static_self: &'static mut Self = unsafe {&mut *(self as *mut _)};

                                ::betterworker::wasm_bindgen_futures::future_to_promise(async move {
                                    static_self._fetch_raw(::betterworker::http::request::from_web_sys_request(req)).await
                                        .map(::betterworker::http::response::into_web_sys_response)
                                        .map(::betterworker::wasm_bindgen::JsValue::from)
                                        .map_err(::betterworker::wasm_bindgen::JsValue::from)
                                })
                            }
                            #method
                        }
                    },
                    "alarm" => {
                        has_alarm = true;

                        let mut method = impl_method.clone();
                        method.sig.ident = Ident::new("_alarm_raw", method.sig.ident.span());
                        quote! {
                            #pound[wasm_bindgen::prelude::wasm_bindgen(js_name = alarm)]
                            pub fn _alarm(&mut self) -> ::betterworker::js_sys::Promise {
                                // SAFETY:
                                // On the surface, this is unsound because the Durable Object could be dropped
                                // while JavaScript still has possession of the future. However,
                                // we know something that Rust doesn't: that the Durable Object will never be destroyed
                                // while there is still a running promise inside of it, therefore we can let a reference
                                // to the durable object escape into a static-lifetime future.
                                let static_self: &'static mut Self = unsafe {&mut *(self as *mut _)};

                                ::betterworker::wasm_bindgen_futures::future_to_promise(async move {
                                    static_self._alarm_raw().await
                                        .map(::betterworker::http::response::into_web_sys_response)
                                        .map(::betterworker::wasm_bindgen::JsValue::from)
                                        .map_err(::betterworker::wasm_bindgen::JsValue::from)
                                })
                            }

                            #method
                        }
                    },
                    _ => panic!(),
                };
                tokenized.push(tokens);
            }

            let alarm_tokens = has_alarm.then(|| quote! {
                async fn alarm(
                    &mut self,
                ) -> ::betterworker::result::Result<::betterworker::http::Response<::betterworker::body::Body>> {
                    self._alarm_raw().await
                }
            });
            Ok(quote! {
                #wasm_bindgen_attr
                impl #struct_name {
                    #(#tokenized)*
                }

                impl ::betterworker::durable::DurableObject for #struct_name {
                    fn new(state: ::betterworker::durable::State, env: ::betterworker::env::Env) -> Self {
                        Self::_new(state._inner(), env._inner())
                    }

                    async fn fetch(
                        &mut self, req: ::betterworker::http::Request<::betterworker::body::Body>,
                    ) -> ::betterworker::result::Result<::betterworker::http::Response<::betterworker::body::Body>> {
                        self._fetch_raw(req).await
                    }

                    #alarm_tokens
                }

                trait __Need_Durable_Object_Trait_Impl_With_durable_object_Attribute { const MACROED: bool = true; }
                impl __Need_Durable_Object_Trait_Impl_With_durable_object_Attribute for #struct_name {}
            })
        },
        Item::Struct(struc) => {
            let tokens = struc.to_token_stream();
            let pound = syn::Token![#](struc.span()).to_token_stream();
            let struct_name = struc.ident;
            Ok(quote! {
                #pound[::betterworker::wasm_bindgen::prelude::wasm_bindgen]
                #tokens

                const _: bool = <#struct_name as __Need_Durable_Object_Trait_Impl_With_durable_object_Attribute>::MACROED;
            })
        },
        _ => Err(Error::new(
            item.span(),
            "Durable Object macro can only be applied to structs and their impl of DurableObject \
             trait",
        )),
    }
}
