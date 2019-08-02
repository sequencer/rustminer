use bytes::{BufMut, BytesMut};
use futures::stream::Stream;
use futures::{Async, Poll};
use num_traits::cast::ToPrimitive;
use tokio_serial::{ClearBuffer, SerialPort};

use crate::stratum::Params;
use crate::util::ToHex;

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
    pub fn target(&self, nonce: u32) -> Bytes {
        let mut target = BytesMut::with_capacity(80);
        target.extend(&self.block_header);
        debug_assert_eq!(target.len(), 76);
        target.put_u32_be(nonce);

        target = target.flip32().sha256d();
        target.reverse();

        target.freeze()
    }

    pub fn target_diff(target: &Bytes) -> f64 {
        2.695_994_666_715_064e67 / BigUint::from_bytes_be(target).to_f64().unwrap()
    }

    pub fn diff(&self, nonce: u32) -> f64 {
        let target = self.target(nonce);
        Self::target_diff(&target)
    }

    pub fn into_params(self, name: &str, nonce: u32) -> Params {
        Params::Submit([
            String::from(name),
            self.workid,
            self.xnonce2.to_hex(),
            self.block_header[68..72].to_hex(),
            nonce.to_be_bytes().to_hex(),
        ])
    }
}

pub struct SubworkMaker {
    work: Work,
    xnonce1: Bytes,
    xnonce2_size: usize,
    counter: BigUint,
    serial_cloned: Box<dyn SerialPort>,
    work_notify: Notify,
}

impl SubworkMaker {
    pub fn new(
        work: Work,
        xnonce: &(Bytes, usize),
        work_notify: Notify,
        serial_cloned: Box<dyn SerialPort>,
    ) -> Self {
        work_notify.notified();
        Self {
            work,
            xnonce1: Bytes::from(xnonce.0.as_ref()),
            xnonce2_size: xnonce.1,
            counter: BigUint::from(0u32),
            serial_cloned,
            work_notify,
        }
    }

    fn next(&mut self) -> Option<Subwork> {
        if self.work_notify.notified() {
            if self.serial_cloned.clear(ClearBuffer::Output).is_ok() {
                debug!("serial buffer cleared!");
            };
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

        Some(self.work.subwork((&self.xnonce1, xnonce2)))
    }
}

impl Stream for SubworkMaker {
    type Item = Subwork;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(Async::Ready(self.next()))
    }
}
