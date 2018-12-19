use std::fmt;
use std::ops::Deref;
use core::slice::Iter;

use hex::{self, FromHex};
use bytes::Bytes;
use serde::{de, Deserializer, Deserialize};

pub struct BytesFromHex(Bytes);

impl Deref for BytesFromHex {
    type Target = Bytes;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> IntoIterator for &'a BytesFromHex {
    type Item = &'a u8;
    type IntoIter = Iter<'a, u8>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_ref().iter()
    }
}

struct BytesVisitor;

impl<'de> de::Visitor<'de> for BytesVisitor {
    type Value = Vec<u8>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bytes from hex")
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Vec::from_hex(v).unwrap_or_default())
    }
}

impl<'de> Deserialize<'de> for BytesFromHex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where
        D: Deserializer<'de> {
        let buf = deserializer.deserialize_str(BytesVisitor)?;
        Ok(BytesFromHex(Bytes::from(buf)))
    }
}
