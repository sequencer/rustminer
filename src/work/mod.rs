use bytes::Bytes;
use num_bigint::BigUint;
use serde::{Serialize, Deserialize};

use super::utils::*;
pub use self::subwork::*;

mod subwork;
#[cfg(test)]
mod tests;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Work {
    id: Bytes,
    #[serde(deserialize_with = "hex_to::bytes")]
    prevhash: Bytes,
    #[serde(deserialize_with = "hex_to::bytes")]
    coinbase1: Bytes,
    #[serde(deserialize_with = "hex_to::bytes")]
    coinbase2: Bytes,
    #[serde(deserialize_with = "hex_to::bytes_vec")]
    merkle_branch: Vec<Bytes>,
    #[serde(deserialize_with = "hex_to::bytes")]
    version: Bytes,
    #[serde(deserialize_with = "hex_to::bytes")]
    nbits: Bytes,
    #[serde(deserialize_with = "hex_to::bytes")]
    ntime: Bytes,
    clean: bool,
}

impl Work {
    fn merkle_root(&self, xnonce: &Bytes) -> Bytes {
        let mut coinbase = Bytes::with_capacity(250);
        coinbase.extend(&self.coinbase1);
        coinbase.extend(xnonce);
        coinbase.extend(&self.coinbase2);
        let mut root = sha256d(&coinbase);
        for node in &self.merkle_branch {
            root.extend(node);
            root = sha256d(&root);
        }
        flip32(root)
    }

    pub fn block_header(&self, xnonce: &Bytes) -> Bytes {
        let mut ret = Bytes::with_capacity(76);
        ret.extend(&self.version);
        ret.extend(&self.prevhash);
        ret.extend(&self.merkle_root(xnonce));
        ret.extend(&self.ntime);
        ret.extend(&self.nbits);
        ret
    }

    pub fn subwork(&self, xnonce: &Bytes) -> SubWork {
        let block_header = self.block_header(xnonce);
        SubWork {
            midstate: sha256_midstate(&block_header[..64]),
            data2: Bytes::from(&block_header[64..]),
            block_header,
            nonce: None,
        }
    }
}
