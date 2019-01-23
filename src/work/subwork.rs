use super::*;

use futures::stream::Stream;
use futures::{Async, Poll};

#[allow(dead_code)]
#[derive(Debug)]
pub struct SubWork {
    pub midstate: Bytes,
    pub data2: Bytes,
    pub block_header: Bytes,
    pub nonce: Option<Bytes>,
}

#[allow(dead_code)]
impl SubWork {
    pub fn send_to_asic(&self) {
        unimplemented!();
    }

    pub fn diff(&self) -> BigUint {
        static NUM: [u32; 7] = [0xffffffffu32; 7];
        let mut temp = Bytes::new();
        temp.extend(&self.block_header);
        temp.extend(self.nonce.as_ref().unwrap());
        BigUint::from_slice(&NUM) / BigUint::from_bytes_be(flip32(temp).as_ref())
    }

    pub fn recv_nonce(&mut self, nonce: Bytes) {
        self.nonce = Some(nonce);
    }
}

#[derive(Debug)]
pub struct SubWorkMaker {
    work: Work,
    xnonce1: Bytes,
    xnonce2_size: usize,
    counter: BigUint,
}

impl SubWorkMaker {
    pub fn new(work: Work, xnonce: &(Bytes, usize)) -> Self {
        Self {
            work,
            xnonce1: Bytes::from(xnonce.0.as_ref()),
            xnonce2_size: xnonce.1,
            counter: BigUint::from(0u32),
        }
    }

    fn next(&mut self) -> Option<SubWork> {
        if self.xnonce2_size < self.counter.bits() {
            return None;
        }
        let size_diff = self.xnonce2_size - self.counter.bits();

        let mut xnonce = Bytes::with_capacity(16);
        xnonce.extend(&self.xnonce1);
        xnonce.extend(vec![0u8; size_diff]);
        xnonce.extend(self.counter.to_bytes_be());
        self.counter += 1u32;
        Some(self.work.subwork(&xnonce))
    }
}

impl Stream for SubWorkMaker {
    type Item = SubWork;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.next() {
            Some(sw) => Ok(Async::Ready(Some(sw))),
            None => Ok(Async::Ready(None))
        }
    }
}
