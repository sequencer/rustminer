use super::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Action {
    pub id: Option<u32>,
    pub method: String,
    pub params: Params,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Respond {
    pub id: Option<u32>,
    pub result: serde_json::Value,
    pub error: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Params {
    Work(Work),
    Bool(bool),
    Difficulty([u32; 1]),
    User([String; 2]),
    None(Vec<()>),
}

pub trait ToString: serde::Serialize {
    fn to_string(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self)
    }
}

impl<T: serde::Serialize> ToString for T {}
