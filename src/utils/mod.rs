use ring::digest;
use bytes::Bytes;

pub mod hex;
pub mod serial;
pub use self::hex::FromHex;

pub fn sha256d(data: &Bytes) -> Bytes {
    let mut data = digest::digest(&digest::SHA256, data);
    data = digest::digest(&digest::SHA256, data.as_ref());
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
    use std::mem::transmute;

    #[allow(dead_code)]
    struct Context {
        state: [u64; digest::MAX_CHAINING_LEN / 8],
        completed_data_blocks: u64,
        pending: [u8; digest::MAX_BLOCK_LEN],
        num_pending: usize,
        pub algorithm: &'static digest::Algorithm,
    }

    let mut ctx = digest::Context::new(&digest::SHA256);
    let data = Bytes::from(data);
    ctx.update(flip32(data).as_ref());

    let mut state = [0u32; 8];
    state.copy_from_slice(
        unsafe { &transmute::<_, [u32; 16]>(transmute::<_, Context>(ctx).state)[..8] }
    );

    let mut ret = Bytes::with_capacity(32);
    for i in state.iter() {
        ret.extend(&i.to_le_bytes());
    }
    ret
}

pub mod hex_to {
    use super::*;
    use serde::{de, Deserializer, Deserialize};

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
