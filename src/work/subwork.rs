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
pub struct Subwork {
    pub workid: String,
    pub midstate: Bytes,
    pub data2: Bytes,
    pub block_header: Bytes,
    pub xnonce2: Bytes,
}

impl Subwork {
    pub fn target(&self, nonce: &Bytes) -> Bytes {
        let mut target = BytesMut::with_capacity(32);
        target.extend(&self.block_header);
        target.extend(nonce);
        target = target.flip32().sha256d();
        target.reverse();
        target.freeze()
    }

    pub fn target_diff(target: &Bytes) -> f64 {
        2.695_994_666_715_064e67 / BigUint::from_bytes_be(target).to_f64().unwrap()
    }

    pub fn diff(&self, nonce: &Bytes) -> f64 {
        let target = self.target(nonce);
        Self::target_diff(&target)
    }

    pub fn into_params(self, name: &str, nonce: &Bytes) -> Params {
        Params::Submit([
            String::from(name),
            self.workid,
            self.xnonce2.to_hex(),
            self.block_header[68..72].to_hex(),
            nonce.to_hex(),
        ])
    }
}

pub struct SubworkMaker {
    work: Work,
    xnonce1: Bytes,
    xnonce2_size: usize,
    counter: BigUint,
    serial_cloned: Box<SerialPort>,
    has_new_work: Arc<Mutex<Option<()>>>,
}

impl SubworkMaker {
    pub fn new(
        work: Work,
        xnonce: &(Bytes, usize),
        has_new_work: Arc<Mutex<Option<()>>>,
        serial_cloned: Box<SerialPort>,
    ) -> Self {
        has_new_work.lock().unwrap().take();
        Self {
            work,
            xnonce1: Bytes::from(xnonce.0.as_ref()),
            xnonce2_size: xnonce.1,
            counter: BigUint::from(0u32),
            serial_cloned,
            has_new_work,
        }
    }

    fn next(&mut self) -> Option<Subwork> {
        if self.xnonce2_size * 8 < self.counter.bits() {
            return None;
        }

        if self.has_new_work.lock().unwrap().take().is_some() {
            if self.serial_cloned.clear(ClearBuffer::Output).is_ok() {
                println!("serial buffer cleared!");
            };
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
