use bytes::Bytes;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

mod subwork;
mod subwork2;
#[cfg(test)]
mod tests;

pub use self::subwork::*;
pub use self::subwork2::*;
use super::util::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct Work {
    pub id: String,
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
    fn merkle_root(&self, xnonce: &(&Bytes, Bytes)) -> Bytes {
        let mut coinbase = Bytes::with_capacity(250);
        coinbase.extend(&self.coinbase1);
        coinbase.extend(xnonce.0);
        coinbase.extend(&xnonce.1);
        coinbase.extend(&self.coinbase2);
        let mut root = coinbase.sha256d();
        for node in &self.merkle_branch {
            root.extend(node);
            root = root.sha256d();
        }
        root.flip32()
    }

    pub fn block_header(&self, xnonce: &(&Bytes, Bytes)) -> Bytes {
        let mut header = Bytes::with_capacity(76);
        header.extend(&self.version.to_be_bytes());
        header.extend(&self.prevhash);
        header.extend(&self.merkle_root(xnonce));
        header.extend(&self.ntime);
        header.extend(&self.nbits);
        header
    }

    pub fn subwork(&self, xnonce: (&Bytes, Bytes)) -> Subwork {
        let block_header = self.block_header(&xnonce);
        Subwork {
            workid: self.id.clone(),
            midstate: sha256_midstate(&block_header[..64]),
            data2: Bytes::from(&block_header[64..]),
            block_header,
            xnonce2: xnonce.1,
        }
    }

    pub fn subwork2(&self, xnonce: (&Bytes, Bytes), vermask: u32) -> Subwork2 {
        Subwork2 {
            workid: self.id.clone(),
            prevhash: self.prevhash.clone(),
            merkle_root: self.merkle_root(&xnonce),
            ntime: self.ntime.clone(),
            nbits: self.nbits.clone(),
            xnonce2: xnonce.1,
            version: self.version,
            vermask,
        }
    }
}
