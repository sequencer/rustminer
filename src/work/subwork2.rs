use bytes::BytesMut;
use futures::stream::Stream;
use futures::{Async, Poll};
use num_traits::cast::ToPrimitive;

use crate::stratum::Params;
use crate::util::ToHex;

use super::*;

#[derive(Clone, Debug, Default)]
pub struct Subwork2 {
    pub workid: String,
    pub prevhash: Bytes,
    pub merkle_root: Bytes,
    pub ntime: Bytes,
    pub nbits: Bytes,
    pub xnonce2: Bytes,
    pub version: u32,
    pub vermask: u32,
}

impl Subwork2 {
    pub fn block_header(&self, version_bits: u32) -> Bytes {
        let mut header = Bytes::with_capacity(76);

        let version = (self.version & !self.vermask) | version_bits;
        header.extend(&version.to_be_bytes());
        header.extend(&self.prevhash);
        header.extend(&self.merkle_root);
        header.extend(&self.ntime);
        header.extend(&self.nbits);
        header
    }

    pub fn target(&self, nonce: &Bytes, version_bits: u32) -> Bytes {
        let mut target = BytesMut::with_capacity(32);
        target.extend(&self.block_header(version_bits));
        target.extend(nonce);
        target = target.flip32().sha256d();
        target.reverse();
        target.freeze()
    }

    pub fn target_diff(target: &Bytes) -> f64 {
        2.695_994_666_715_064e67 / BigUint::from_bytes_be(target).to_f64().unwrap()
    }

    pub fn into_params(self, name: &str, nonce: &Bytes, version_bits: u32) -> Params {
        Params::Submit2([
            String::from(name),
            self.workid,
            self.xnonce2.to_hex(),
            self.ntime.to_hex(),
            nonce.to_hex(),
            version_bits.to_be_bytes().to_hex(),
        ])
    }
}

pub struct Subwork2Maker {
    work: Work,
    xnonce1: Bytes,
    xnonce2_size: usize,
    vermask: u32,
    counter: BigUint,
    work_notify: Notify,
}

impl Subwork2Maker {
    pub fn new(work: Work, xnonce: &(Bytes, usize), vermask: u32, work_notify: Notify) -> Self {
        work_notify.notified();
        Self {
            work,
            xnonce1: Bytes::from(xnonce.0.as_ref()),
            xnonce2_size: xnonce.1,
            vermask,
            counter: BigUint::from(0u32),
            work_notify,
        }
    }

    fn next(&mut self) -> Option<Subwork2> {
        if self.work_notify.notified() {
            return None;
        }

        if self.xnonce2_size * 8 < self.counter.bits() {
            return None;
        }

        let xnonce2_tail = self.counter.to_bytes_be();
        let size_diff = self.xnonce2_size - xnonce2_tail.len();

        let mut xnonce2 = Bytes::with_capacity(self.xnonce2_size);
        xnonce2.extend(vec![0u8; size_diff]);
        xnonce2.extend(xnonce2_tail);

        self.counter += 1u8;

        Some(self.work.subwork2((&self.xnonce1, xnonce2), self.vermask))
    }
}

impl Stream for Subwork2Maker {
    type Item = Subwork2;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(Async::Ready(self.next()))
    }
}
