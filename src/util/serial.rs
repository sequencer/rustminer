use std::path::Path;

use bytes::{Bytes, BytesMut, BufMut};
use tokio::io;
use tokio_codec::{Decoder, Encoder, Framed};
use tokio_serial::{Serial, SerialPortSettings};
use crc::Crc;
use lazy_static::lazy_static;

use super::super::work::Subwork;

fn crc5usb(data: &[u8]) -> u8 {
    lazy_static!(static ref CRC5_USB: Crc<u8> = Crc::<u8>::new(0x05, 5, 0x1f, 0x1f, true););
    let crc = &mut 0u8;
    CRC5_USB.init_crc(crc);
    CRC5_USB.update_crc(crc, data)
}

fn crc5usb_check(data: &[u8]) -> bool {
    crc5usb(data) == 1
}

fn crc16_ccitt_false(data: &[u8]) -> u16 {
    lazy_static!(static ref CRC16_CCITT_FALSE: Crc<u16> = Crc::<u16>::new(0x1021, 16, 0xffff, 0, false););
    let crc = &mut 0u16;
    CRC16_CCITT_FALSE.init_crc(crc);
    CRC16_CCITT_FALSE.update_crc(crc, data)
}

fn _print_hex(data: &[u8]) {
    print!("0x");
    for b in data {
        print!("{:02x}", b);
    }
    println!();
}

#[derive(Debug)]
pub struct Codec {
    subworkid: u8,
    subworks: Vec<Option<Subwork>>,
}

impl Default for Codec {
    fn default() -> Self {
        Self {
            subworkid: 0,
            subworks: vec![None; 256],
        }
    }
}

impl Decoder for Codec {
    type Item = (Subwork, Bytes);
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<<Self as Decoder>::Item>, <Self as Decoder>::Error> {
        if let Some(n) = src.iter().position(|b| *b == 0x55) {
            if src.len() >= n + 7 {
                let item = &src[n..n + 7];
                _print_hex(item);
                if crc5usb_check(item) {
                    let received = src.split_to(n + 7).split_off(n);
                    let subworkid = received[5];

                    let subwork = self.subworks[subworkid as usize].take();
                    let mut dst = BytesMut::new();

                    if let Some(sw) = &subwork {
                        dst.extend(b"\x20\x31");
                        dst.put_u8(subworkid);
                        dst.extend(sw.data2.iter().rev());
                        dst.extend(sw.midstate.iter().rev());
                        dst.extend(&crc16_ccitt_false(dst.as_ref()).to_be_bytes());
                        print!("subwork: ");
                        _print_hex(dst.as_ref());
                    }

                    let nonce = &received[2..6];
                    return Ok(subwork.map(|sw|(sw, Bytes::from(nonce))));
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
        dst.extend(b"\x20\x31");
        dst.put_u8(self.subworkid);
        dst.extend(item.data2.iter().rev());
        dst.extend(item.midstate.iter().rev());
        dst.extend(&crc16_ccitt_false(dst.as_ref()).to_be_bytes());
        self.subworks[self.subworkid as usize] = Some(item);
        self.subworkid = self.subworkid.wrapping_add(1);
        // debug
//        print!("subwork: ");
//        _print_hex(dst.as_ref());
        Ok(())
    }
}

pub fn serial_framed<T: AsRef<Path>>(path: T) -> Framed<Serial, Codec> {
    let mut s = SerialPortSettings::default();
    s.baud_rate = 115_200;

    let mut port = Serial::from_path(path, &s).unwrap();
    #[cfg(unix)]
        port.set_exclusive(false).expect("set_exclusive(false) failed!");

    Codec::default().framed(port)
}

#[test]
fn serial_receive() {
    use tokio::prelude::*;

    #[cfg(unix)]
    const PORT: &str = "/dev/ttyUSB0";
    #[cfg(windows)]
    const PORT: &str = "COM1";

//    let (_, reader) = Codec.framed(
//        tokio::fs::File::from_std(std::fs::File::open("/tmp/port").unwrap())
//    ).split();

    let (_, reader) = serial_framed(PORT).split();
    let printer = reader
        .for_each(|s| {
            println!("received: {:?}", s);
            Ok(())
        }).map_err(|e| eprintln!("{}", e));

    tokio::run(printer);
}
