use std::collections::VecDeque;
use std::path::Path;

use bytes::{BufMut, Bytes, BytesMut};
use crc_all::CrcAlgo;
use lazy_static::lazy_static;
use tokio::codec::{Decoder, Encoder, Framed};
use tokio::io;
use tokio_serial::{Serial, SerialPortSettings};

#[allow(unused_imports)]
use crate::util::ToHex;
use crate::work::Subwork;

fn crc5_usb(data: &[u8]) -> u8 {
    lazy_static! {
        static ref CRC5_USB: CrcAlgo<u8> = CrcAlgo::<u8>::new(0x05, 5, 0x1f, 0x1f, true);
    };
    let crc = &mut 0u8;
    CRC5_USB.init_crc(crc);
    CRC5_USB.update_crc(crc, data)
}

fn crc5_usb_check(data: &[u8]) -> bool {
    crc5_usb(data) == 1
}

fn crc16_ccitt_false(data: &[u8]) -> u16 {
    lazy_static! {
        static ref CRC16_CCITT_FALSE: CrcAlgo<u16> =
            CrcAlgo::<u16>::new(0x1021, 16, 0xffff, 0, false);
    };
    let crc = &mut 0u16;
    CRC16_CCITT_FALSE.init_crc(crc);
    CRC16_CCITT_FALSE.update_crc(crc, data)
}

#[derive(Debug)]
pub struct Codec {
    subworkid: u8,
    subworks: Vec<Option<Subwork>>,
    received: VecDeque<BytesMut>,
}

impl Default for Codec {
    fn default() -> Self {
        Self {
            subworkid: 0,
            subworks: vec![None; 256],
            received: VecDeque::with_capacity(2),
        }
    }
}

impl Decoder for Codec {
    // (subwork, target, nonce)
    type Item = (Subwork, Bytes, u32);
    type Error = io::Error;

    fn decode(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<<Self as Decoder>::Item>, <Self as Decoder>::Error> {
        'outer: loop {
            if let Some(n) = src.iter().position(|b| *b == 0x55) {
                if src.len() >= n + 7 {
                    for received in &self.received {
                        // process next nonce
                        if received.starts_with(&src[n..n + 5]) {
                            let _drop = src.split_to(n + 7);
                            debug!("duplicate data: 0x{}!", _drop.to_hex());
                            continue 'outer;
                        }
                    }

                    if crc5_usb_check(&src[n..n + 7]) {
                        let received = src.split_to(n + 7).split_off(n);
                        let id = received[5];
                        let nonce = u32::from_le_bytes(unsafe {
                            *(received[1..5].as_ptr() as *const [u8; 4])
                        });

                        // check subwork
                        let mut subwork = None;
                        let mut target = Bytes::default();
                        for i in 0..4 {
                            if let Some(ref sw) = &self.subworks[id.wrapping_sub(i) as usize] {
                                target = sw.target(nonce);
                                if target.starts_with(b"\0\0\0\0") {
                                    debug!(
                                        "received: 0x{}, id: {}, target: 0x{}",
                                        received.to_hex(),
                                        id,
                                        target.to_hex()
                                    );
                                    subwork = Some(sw.clone());
                                    break;
                                }
                            }
                        }

                        self.received.push_front(received);
                        self.received.truncate(2);

                        if subwork.is_none() {
                            debug!(
                                "lost the subwork of received data (id: {}): 0x{}",
                                id,
                                self.received.front().unwrap().to_hex()
                            );

                            // process next nonce
                            if src.len() >= 7 {
                                continue 'outer;
                            }
                        }

                        return Ok(subwork.map(|sw| (sw, target, nonce)));
                    } else {
                        src.split_to(n + 1);
                    }
                }
            }
            return Ok(None);
        }
    }
}

impl Encoder for Codec {
    type Item = Subwork;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(49);
        dst.extend(b"\x20\x31");
        dst.put_u8(self.subworkid);
        dst.extend(item.data2.iter().rev());
        dst.extend(item.midstate.iter().rev());
        dst.extend(&crc16_ccitt_false(dst.as_ref()).to_be_bytes());
        self.subworks[self.subworkid as usize] = Some(item);
        self.subworkid = self.subworkid.wrapping_add(1);
        Ok(())
    }
}

pub fn new<T: AsRef<Path>>(path: T) -> Serial {
    let mut s = SerialPortSettings::default();
    s.baud_rate = 1_500_000;

    let mut port = Serial::from_path(path, &s).unwrap();
    #[cfg(unix)]
    port.set_exclusive(false)
        .expect("set_exclusive(false) failed!");

    port
}

pub fn framed(port: Serial) -> Framed<Serial, Codec> {
    Codec::default().framed(port)
}
