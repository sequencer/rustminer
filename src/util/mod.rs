use core::mem::transmute;

use bytes::Bytes;
use sha256::Sha256;

pub mod hex;
pub mod serial;

pub use self::hex::{FromHex, ToHex};

pub fn sha256d(data: &Bytes) -> Bytes {
    let mut sha256 = Sha256::default();
    sha256.update(data);

    let mut data = sha256.finish();

    let mut sha256 = Sha256::default();
    sha256.update(&data);

    data = sha256.finish();

    Bytes::from(data.as_ref())
}

pub fn flip32(data: Bytes) -> Bytes {
    let len = data.len();
    assert_eq!(len % 4, 0);
    let mut data = data.try_mut().unwrap();
    for i in 0..(len / 4) {
        data[i * 4..(i * 4 + 4)].reverse();
    }
    data.freeze()
}

pub fn sha256_midstate(data: &[u8]) -> Bytes {
    let mut sha256 = Sha256::default();

    let data = Bytes::from(data);
    sha256.update(flip32(data).as_ref());

    Bytes::from(unsafe { transmute::<_, [u8; 32]>(sha256.state()).as_ref() })
}

#[allow(dead_code)]
pub fn print_hex(data: &[u8]) {
    print!("0x");
    println!("{}", data.to_hex());
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
}
