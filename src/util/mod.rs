use std::iter::repeat;
use std::iter::FromIterator;

use bytes::{Bytes, BytesMut};
use chrono::Local;
use fern::{Dispatch, InitError};
use sha256::Sha256;

pub use self::{
    hex::{FromHex, ToHex},
    mmap::Mmap,
    notify::Notify,
    sinkhook::SinkHook,
};

pub mod fpga;
pub mod hex;
pub mod i2c;
mod mmap;
mod notify;
pub mod serial;
mod sinkhook;

trait __Flip32: Sized {
    fn __flip32(&mut self);
}

impl<T> __Flip32 for T
where
    T: AsRef<[u8]> + AsMut<[u8]> + Extend<u8>,
{
    fn __flip32(&mut self) {
        let mut len = self.as_ref().len();
        let residue = len % 4;
        if residue > 0 {
            self.extend(repeat(b'\0').take(4 - residue));
            len += 4 - residue;
        }
        for i in 0..(len / 4) {
            self.as_mut()[i * 4..(i * 4 + 4)].reverse();
        }
    }
}

pub trait Flip32: Sized {
    fn flip32(self) -> Self;
}

impl Flip32 for Bytes {
    fn flip32(self) -> Self {
        self.try_mut().unwrap().flip32().freeze()
    }
}

impl Flip32 for BytesMut {
    fn flip32(mut self) -> Self {
        self.__flip32();
        self
    }
}

impl Flip32 for Vec<u8> {
    fn flip32(mut self) -> Self {
        self.__flip32();
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
    use serde::{de, Deserialize, Deserializer};

    use super::*;

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

pub fn setup_logger() -> Result<(), InitError> {
    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{:<5}] {}",
                Local::now().format("[%Y-%m-%d %H:%M:%S%.6f]"),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .level_for("tokio_reactor", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file(format!(
            "/var/log/stratum_{}.log",
            Local::now().format("%Y%m%d_%H%M%S")
        ))?)
        .apply()?;
    Ok(())
}
