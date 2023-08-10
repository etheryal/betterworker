use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends=js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type Fetcher;

    #[wasm_bindgen(method)]
    pub fn fetch(this: &Fetcher, input: &web_sys::Request) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name=fetch)]
    pub fn fetch_with_str(this: &Fetcher, input: &str) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name=fetch)]
    pub fn fetch_with_init(
        this: &Fetcher, input: &web_sys::Request, init: &web_sys::RequestInit,
    ) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name=fetch)]
    pub fn fetch_with_str_and_init(
        this: &Fetcher, input: &str, init: &web_sys::RequestInit,
    ) -> js_sys::Promise;
}
