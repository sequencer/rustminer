use std::cmp::min;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::{BufMut, BytesMut};
use futures::stream::Stream;
use futures::{Async, Poll};
use num_traits::cast::ToPrimitive;

use crate::stratum::*;
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

pub struct PoolData {
    pub duration: Duration,
    pub works: WorkStream,
    pub xnonce: Arc<Mutex<(Bytes, usize)>>,
    pub vermask: Arc<Mutex<Option<u32>>>,
    pub notify: Notify,
    pub maker: Option<Subwork2Maker>,
}

pub struct Subwork2Stream {
    pub pools: Vec<PoolData>,
    pub current_pool: usize,
    pub switch_timeout: Instant,
}

impl Default for Subwork2Stream {
    fn default() -> Self {
        Self {
            pools: Vec::new(),
            current_pool: 0,
            switch_timeout: Instant::now(),
        }
    }
}

impl Subwork2Stream {
    fn current_pool(&mut self) -> usize {
        if self.pools.len() == 2 {
            let now = Instant::now();
            if now > self.switch_timeout {
                self.current_pool ^= 1;
                self.switch_timeout = now + self.pools[self.current_pool].duration;
            }
            debug!("switch to pool {}", self.current_pool);
            self.current_pool
        } else {
            0
        }
    }
}

impl Stream for Subwork2Stream {
    type Item = (Subwork2, Notify, Duration);
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let cur = self.current_pool();

        match self.pools[cur].works.poll() {
            Ok(Async::Ready(Some(work))) => {
                self.pools[cur].notify.notified();
                let subwork2maker = Subwork2Maker::new(
                    work,
                    &self.pools[cur].xnonce.lock().unwrap(),
                    self.pools[cur].vermask.lock().unwrap().unwrap(),
                );
                self.pools[cur].maker = Some(subwork2maker);
            }
            Err(_) => return Err(()),
            _ => {}
        }

        let subwork2 = match self.pools[cur].maker {
            Some(ref mut maker) => maker.next(),
            None => return Ok(Async::NotReady),
        };

        match subwork2 {
            Some(subwork2) => {
                let notify = self.pools[cur].notify.clone();
                let now = Instant::now();
                let timeout = if self.switch_timeout > now {
                    min(Duration::from_secs(10), self.switch_timeout - now)
                } else {
                    Duration::from_secs(10)
                };
                Ok(Async::Ready(Some((subwork2, notify, timeout))))
            }
            None => Ok(Async::NotReady),
        }
    }
}

impl Subwork2 {
    pub fn block_header(&self, version_bits: u32) -> BytesMut {
        let mut header = BytesMut::with_capacity(80);

        let version = (self.version & !self.vermask) | version_bits;
        header.put_u32_be(version);
        header.extend(&self.prevhash);
        header.extend(&self.merkle_root);
        header.extend(&self.ntime);
        header.extend(&self.nbits);
        debug_assert_eq!(header.len(), 76);

        header
    }

    pub fn target(&self, nonce: u32, version_bits: u32) -> Bytes {
        let mut target = self.block_header(version_bits);
        target.put_u32_be(nonce);

        target = target.flip32().sha256d();
        target.reverse();

        target.freeze()
    }

    pub fn target_diff(target: &Bytes) -> f64 {
        2.695_994_666_715_064e67 / BigUint::from_bytes_be(target).to_f64().unwrap()
    }

    pub fn into_params(self, name: &str, nonce: u32, version_bits: u32) -> Params {
        Params::Submit2([
            String::from(name),
            self.workid,
            self.xnonce2.to_hex(),
            self.ntime.to_hex(),
            nonce.to_be_bytes().to_hex(),
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
}

impl Subwork2Maker {
    pub fn new(work: Work, xnonce: &(Bytes, usize), vermask: u32) -> Self {
        Self {
            work,
            xnonce1: Bytes::from(xnonce.0.as_ref()),
            xnonce2_size: xnonce.1,
            vermask,
            counter: BigUint::from(0u32),
        }
    }

    fn next(&mut self) -> Option<Subwork2> {
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
