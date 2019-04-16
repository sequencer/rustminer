use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::map::Map as JsonMap;
use serde_json::Value as JsonValue;

use crate::util::hex_to;

use super::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct Action {
    pub id: Option<u32>,
    pub method: String,
    pub params: Params,
}

#[derive(Deserialize, Debug)]
pub struct Respond {
    pub id: Option<u32>,
    pub result: ResultOf,
    pub error: JsonValue,
}

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Params {
    #[serde(skip_serializing)]
    Work(Work),
    Bool(bool),
    Num([f64; 1]),
    #[serde(skip_serializing, deserialize_with = "hex_to::u32_vec")]
    TMask(Vec<u32>),
    #[serde(skip_deserializing)]
    User([String; 2]),
    #[serde(skip_deserializing)]
    Config(Vec<String>, JsonValue),
    #[serde(skip_deserializing)]
    Submit([String; 5]),
    #[serde(skip_deserializing)]
    Submit2([String; 6]), // with version_bits
    None([(); 0]),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum ResultOf {
    Configure(JsonMap<String, JsonValue>),
    Authorize(Option<bool>),
    Subscribe(
        ResultOfSubscribe,                                  // set_difficulty & notify
        #[serde(deserialize_with = "hex_to::bytes")] Bytes, // xnonce1
        usize,                                              // xnonce2_size
    ),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum ResultOfSubscribe {
    A([String; 2]),
    B([[String; 2]; 2]),
}
