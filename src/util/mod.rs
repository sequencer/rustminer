use std::iter::FromIterator;

use bytes::{Bytes, BytesMut};
use sha256::Sha256;

pub mod fpga;
pub mod hex;
mod mmap;
pub mod serial;
mod sinkhook;

pub use self::hex::{FromHex, ToHex};
pub use self::mmap::Mmap;
pub use sinkhook::SinkHook;

pub trait Flip32: Sized {
    fn flip32(self) -> Self;
}

impl Flip32 for Bytes {
    fn flip32(self) -> Self {
        let data = self.try_mut().unwrap();
        data.flip32().freeze()
    }
}

impl Flip32 for BytesMut {
    fn flip32(mut self) -> Self {
        let len = self.len();
        assert_eq!(len % 4, 0);
        for i in 0..(len / 4) {
            self[i * 4..(i * 4 + 4)].reverse();
        }
        self
    }
}

pub trait Sha256d: Sized + AsRef<[u8]> + for<'a> From<&'a [u8]> {
    fn sha256d(&self) -> Self {
        Self::from(Sha256::digest(&Sha256::digest(self.as_ref())).as_ref())
    }
}

impl Sha256d for Bytes {}

impl Sha256d for BytesMut {}

pub fn sha256_midstate(data: &[u8]) -> Bytes {
    let mut sha256 = Sha256::default();
    sha256.update(Bytes::from(data).flip32().as_ref());

    Bytes::from_iter(sha256.state().iter().flat_map(|x| x.to_le_bytes().to_vec()))
}

pub mod hex_to {
    use super::*;
    use serde::{de, Deserialize, Deserializer};

    pub fn bytes<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Bytes, D::Error> {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Ok(Bytes::from(s.from_hex().map_err(de::Error::custom)?))
    }

    pub fn bytes_vec<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Bytes>, D::Error> {
        let sv: Vec<&str> = Deserialize::deserialize(deserializer)?;
        let mut bv: Vec<Bytes> = Vec::with_capacity(sv.len());
        for s in sv {
            bv.push(Bytes::from(s.from_hex().map_err(de::Error::custom)?));
        }
        Ok(bv)
    }

    pub fn u32<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Ok(u32::from_str_radix(s, 16).map_err(de::Error::custom)?)
    }

    pub fn u32_vec<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u32>, D::Error> {
        let sv: Vec<&str> = Deserialize::deserialize(deserializer)?;
        let mut uv: Vec<u32> = Vec::with_capacity(sv.len());
        for s in sv {
            uv.push(u32::from_str_radix(s, 16).map_err(de::Error::custom)?);
        }
        Ok(uv)
    }
}
