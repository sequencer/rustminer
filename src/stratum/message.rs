use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::super::util::hex_to;
use super::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct Action {
    pub id: Option<u32>,
    pub method: String,
    pub params: Params,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Respond {
    pub id: Option<u32>,
    pub result: ResultOf,
    pub error: serde_json::Value,
}

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Params {
    Work(Work),
    Bool(bool),
    Integer([u32; 1]),
    TMask(TMask),
    User([String; 2]),
    Config(Config),
    Submit([String; 5]),
    None(Vec<()>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TMask(#[serde(deserialize_with = "hex_to::bytes_vec")] Vec<Bytes>);

#[derive(Serialize, Deserialize, Debug)]
pub struct Config(pub Vec<String>, pub serde_json::Value);

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ResultOf {
    Configure(serde_json::map::Map<String, serde_json::Value>),
    Authorize(bool),
    Subscribe(ResultOfSubscribe),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResultOfSubscribe(
    pub [StringWithBytes; 2], // set_difficulty & notify
    #[serde(deserialize_with = "hex_to::bytes")] pub Bytes, // xnonce1
    pub usize,                // xnonce2_size
);

#[derive(Serialize, Deserialize, Debug)]
pub struct StringWithBytes(String, #[serde(deserialize_with = "hex_to::bytes")] Bytes);

pub trait ToJsonString: serde::Serialize {
    fn to_string(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self)
    }
}

impl<T: serde::Serialize> ToJsonString for T {}
