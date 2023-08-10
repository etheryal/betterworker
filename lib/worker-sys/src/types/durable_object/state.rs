use wasm_bindgen::prelude::*;

use crate::types::durable_object::{DurableObjectId, DurableObjectStorage};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends=js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type DurableObjectState;

    #[wasm_bindgen(method, getter)]
    pub fn id(this: &DurableObjectState) -> DurableObjectId;

    #[wasm_bindgen(method, getter)]
    pub fn storage(this: &DurableObjectState) -> DurableObjectStorage;

    #[wasm_bindgen(method, js_name=blockConcurrencyWhile)]
    pub fn block_concurrency_while(this: &DurableObjectState, promise: &js_sys::Promise);
}
