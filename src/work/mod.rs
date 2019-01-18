use bytes::Bytes;
use num_bigint::BigUint;
use serde::{Serialize, Deserialize};

use super::util::*;
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
    #[serde(deserialize_with = "hex_to::u32")]
    version: u32,
    #[serde(deserialize_with = "hex_to::bytes")]
    nbits: Bytes,
    #[serde(deserialize_with = "hex_to::bytes")]
    ntime: Bytes,
    pub clean: bool,
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
        ret.extend(&self.version.to_be_bytes());
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

pub struct Chunk1Itor {
    counter: u32,
    version: u32,
    vermask: u32,
    offset: u32,
    rsize: u32,
    tail: Bytes,
}

impl Chunk1Itor {
    pub fn new(work: &Work, xnonce: &Bytes, vermask: u32) -> Self {
        let offset = vermask.trailing_zeros();
        let rsize = vermask.leading_zeros() + offset;
        let mut tail = Bytes::with_capacity(60);
        tail.extend(&work.prevhash);
        tail.extend(&work.merkle_root(xnonce)[..28]);
        Self {
            counter: 0,
            version: work.version,
            vermask,
            offset,
            rsize,
            tail,
        }
    }
}

impl Iterator for Chunk1Itor {
    type Item = (u32, Bytes);

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.counter.leading_zeros() >= self.rsize {
            self.counter += 1u32;
            let version_bits = self.counter << self.offset;
            let version = (self.version | self.vermask) & version_bits;
            let mut chunk1 = Bytes::with_capacity(64);
            chunk1.extend(&version.to_be_bytes());
            chunk1.extend(&self.tail);
            Some((version_bits, chunk1))
        } else {
            None
        }
    }
}
