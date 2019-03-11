use std::sync::{Arc, Mutex};

use bytes::BytesMut;
use futures::stream::Stream;
use futures::{Async, Poll};
use num_traits::cast::ToPrimitive;
use tokio_serial::{ClearBuffer, SerialPort};

use super::*;
use crate::stratum::Params;
use crate::util::ToHex;

#[derive(Clone, Debug, Default)]
pub struct Subwork2 {
    pub workid: String,
    pub prevhash: Bytes,
    pub merkle_root: Bytes,
    pub ntime: Bytes,
    pub xnonce2: Bytes,
    pub vermask: u32,
}

impl Subwork2 {
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
    has_new_work: Arc<Mutex<Option<()>>>,
}

impl Subwork2Maker {
    pub fn new(
        work: Work,
        xnonce: &(Bytes, usize),
        vermask: u32,
        has_new_work: Arc<Mutex<Option<()>>>,
    ) -> Self {
        has_new_work.lock().unwrap().take();
        Self {
            work,
            xnonce1: Bytes::from(xnonce.0.as_ref()),
            xnonce2_size: xnonce.1,
            vermask,
            counter: BigUint::from(0u32),
            has_new_work,
        }
    }

    fn next(&mut self) -> Option<Subwork2> {
        if self.has_new_work.lock().unwrap().take().is_some() {
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
