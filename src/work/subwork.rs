use futures::stream::Stream;
use futures::{Async, Poll};

use super::super::stratum::Params;
use super::super::util::hex::ToHex;
use super::*;

#[derive(Clone, Debug, Default)]
pub struct Subwork {
    pub workid: String,
    pub midstate: Bytes,
    pub data2: Bytes,
    pub block_header: Bytes,
    pub xnonce2: Bytes,
}

impl Subwork {
    pub fn diff(&self, nonce: &Bytes) -> BigUint {
        static NUM: [u32; 7] = [0xffff_ffff; 7];
        let mut temp = Bytes::new();
        temp.extend(&self.block_header);
        temp.extend(nonce);
        BigUint::from_slice(&NUM) / BigUint::from_bytes_be(flip32(temp).as_ref())
    }

    pub fn into_params(self, name: &str, nonce: Bytes) -> Params {
        Params::Submit([
            String::from(name),
            self.workid,
            format!("{:0>8}", self.xnonce2.to_hex()),
            self.block_header[68..72].to_hex(),
            nonce.to_hex(),
        ])
    }
}

#[derive(Debug)]
pub struct SubworkMaker {
    work: Work,
    xnonce1: Bytes,
    xnonce2_size: usize,
    counter: BigUint,
}

impl SubworkMaker {
    pub fn new(work: Work, xnonce: &(Bytes, usize)) -> Self {
        Self {
            work,
            xnonce1: Bytes::from(xnonce.0.as_ref()),
            xnonce2_size: xnonce.1,
            counter: BigUint::from(0u32),
        }
    }

    fn next(&mut self) -> Option<Subwork> {
        if self.xnonce2_size * 8 < self.counter.bits() {
            return None;
        }

        let xnonce2_tail = self.counter.to_bytes_be();
        let size_diff = self.xnonce2_size - xnonce2_tail.len();

        let mut xnonce2 = Bytes::with_capacity(self.xnonce2_size);
        xnonce2.extend(vec![0u8; size_diff]);
        xnonce2.extend(xnonce2_tail);

        self.counter += 1u8;

        Some(self.work.subwork((&self.xnonce1, xnonce2)))
    }
}

impl Stream for SubworkMaker {
    type Item = Subwork;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.next() {
            Some(sw) => Ok(Async::Ready(Some(sw))),
            None => Ok(Async::Ready(None)),
        }
    }
}
