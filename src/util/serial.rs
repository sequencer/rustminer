use std::path::Path;

use bytes::{BytesMut, BufMut};
use tokio::io;
use tokio_codec::{Decoder, Encoder, Framed};
use tokio_serial::{Serial, FlowControl, SerialPortSettings};
use crc::Crc;
use lazy_static::lazy_static;

use super::super::work::SubWork;

fn crc5usb(data: &[u8]) -> u8 {
    lazy_static!(static ref CRC5_USB: Crc<u8> = Crc::<u8>::new(0x05, 5, 0x1f, 0x1f, true););
    let crc = &mut 0u8;
    CRC5_USB.init_crc(crc);
    CRC5_USB.update_crc(crc, data)
}

fn crc5usb_check(data: &[u8]) -> bool {
    if crc5usb(data) == 1 {
        true
    } else {
        false
    }
}

fn crc16_ccitt_false(data: &[u8]) -> u16 {
    lazy_static!(static ref CRC16_CCITT_FALSE: Crc<u16> = Crc::<u16>::new(0x1021, 16, 0xffff, 0, false););
    let crc = &mut 0u16;
    CRC16_CCITT_FALSE.init_crc(crc);
    CRC16_CCITT_FALSE.update_crc(crc, data)
}

#[derive(Debug)]
pub struct Codec {
    workid: u8
}

impl Codec {
    pub fn new() -> Self {
        Self { workid: 0 }
    }
}

impl Decoder for Codec {
    type Item = BytesMut;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<<Self as Decoder>::Item>, <Self as Decoder>::Error> {
//        fn print_hex(data: &[u8]) {
//            print!("{}", "0x");
//            for b in data {
//                print!("{:02x}", b);
//            }
//            println!();
//        }
        if let Some(n) = src.iter().position(|b| *b == 0x55) {
            if src.len() >= n + 7 {
                let item = &src[n..n + 7];
//                print_hex(item);
                if crc5usb_check(item) {
                    return Ok(Some(src.split_to(n + 7).split_off(n)));
                } else {
                    src.split_to(n);
                }
            }
        }
        Ok(None)
    }
}

impl Encoder for Codec {
    type Item = SubWork;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend(b"\x20\x31");
        dst.put_u8(self.workid);
        self.workid = self.workid.wrapping_add(1);
        dst.extend(&item.data2);
        dst.extend(&item.midstate);
        dst.extend(&crc16_ccitt_false(dst.as_ref()).to_be_bytes());
        Ok(())
    }
}

pub fn serial_framed<T: AsRef<Path>>(path: T) -> Framed<Serial, Codec> {
    let mut s = SerialPortSettings::default();
    s.baud_rate = 115200;
    s.flow_control = FlowControl::Software;

    let mut port = Serial::from_path(path, &s).unwrap();
    #[cfg(unix)]
        port.set_exclusive(false).expect("set_exclusive(false) failed!");

    Codec::new().framed(port)
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
            println!("received {} bytes: {:?}", s.len(), s);
            Ok(())
        }).map_err(|e| eprintln!("{}", e));

    tokio::run(printer);
}
