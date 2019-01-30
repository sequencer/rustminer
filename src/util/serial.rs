use std::iter::FromIterator;
use std::path::Path;

use bytes::{BufMut, Bytes, BytesMut};
use crc::CrcAlgo;
use lazy_static::lazy_static;
use tokio::io;
use tokio_codec::{Decoder, Encoder, Framed};
use tokio_serial::{Serial, SerialPortSettings};

#[allow(unused_imports)]
use super::super::util::print_hex;
use super::super::work::Subwork;

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
    subworkid: usize,
    subworks: Vec<Option<Subwork>>,
    received: BytesMut,
}

impl Default for Codec {
    fn default() -> Self {
        Self {
            subworkid: 0,
            subworks: vec![None; 0x3fff + 1],
            received: BytesMut::new(),
        }
    }
}

impl Decoder for Codec {
    type Item = (Subwork, Bytes);
    type Error = io::Error;

    fn decode(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<<Self as Decoder>::Item>, <Self as Decoder>::Error> {
        if let Some(n) = src.iter().position(|b| *b == 0x55) {
            if src.len() >= n + 7 {
                let item = &src[n..n + 7];
                if item == self.received.as_ref() {
                    drop(src.split_to(n + 7));
                    return Ok(None);
                }

                if crc5_usb_check(item) {
                    self.received = src.split_to(n + 7).split_off(n);
                    let id = self.received[5] as usize;
                    let nonce = Bytes::from_iter(self.received[1..5].iter().rev().map(|b| *b));

                    let mut subworkid;
                    let mut subwork = None;
                    for i in 0..0x3fusize {
                        subworkid = i << 8 | id;
                        match &self.subworks[subworkid] {
                            Some(ref sw) => {
                                let target = sw.target(&nonce);

                                // debug
                                print!("check target: ");
                                print_hex(&target);

                                if target.starts_with(b"\0\0\0\0") {
                                    subwork = self.subworks[subworkid].take();
                                    break;
                                }
                            }
                            _ => (),
                        }
                    }

                    // debug
                    if subwork.is_none() {
                        eprint!("!!! lost the subwork of nonce: ");
                        print_hex(&nonce);
                    }

                    return Ok(subwork.map(|sw| (sw, nonce)));
                } else {
                    src.split_to(n);
                }
            }
        }
        Ok(None)
    }
}

impl Encoder for Codec {
    type Item = Subwork;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(49);
        dst.extend(b"\x20\x31");
        dst.put_u8(self.subworkid.to_le_bytes()[0]);
        dst.extend(item.data2.iter().rev());
        dst.extend(item.midstate.iter().rev());
        dst.extend(&crc16_ccitt_false(dst.as_ref()).to_be_bytes());
        self.subworks[self.subworkid & 0x3fff] = Some(item);
        self.subworkid = self.subworkid.wrapping_add(1);
        Ok(())
    }
}

pub fn serial_framed<T: AsRef<Path>>(path: T) -> Framed<Serial, Codec> {
    let mut s = SerialPortSettings::default();
    s.baud_rate = 115_200;

    let mut port = Serial::from_path(path, &s).unwrap();
    #[cfg(unix)]
    port.set_exclusive(false)
        .expect("set_exclusive(false) failed!");

    Codec::default().framed(port)
}

#[test]
fn serial_receive() {
    use tokio::prelude::*;

    #[cfg(unix)]
    const PORT: &str = "/dev/ttyUSB0";
    #[cfg(windows)]
    const PORT: &str = "COM1";

    let (_, reader) = serial_framed(PORT).split();
    let printer = reader
        .for_each(|s| {
            println!("received: {:?}", s);
            Ok(())
        })
        .map_err(|e| eprintln!("{}", e));

    tokio::run(printer);
}
