use std::io::BufReader;
use std::path::Path;

use bytes::BytesMut;
use tokio::io;
use tokio_codec::{Decoder, Encoder};
use tokio_serial::{Serial, FlowControl, SerialPortSettings};
use crc::Crc;
use lazy_static::lazy_static;

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

struct Codec;

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
                let item = &src[n..n+7];
//                print_hex(item);
                if crc5usb_check(item) {
                    return Ok(Some(src.split_to(n+7).split_off(n)));
                } else {
                    src.split_to(n);
                }
            }
        }
        Ok(None)
    }
}

impl Encoder for Codec {
    type Item = BytesMut;
    type Error = io::Error;

    fn encode(&mut self, _item: Self::Item, _dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn serial_receive() {
    use tokio::prelude::*;

    #[cfg(unix)]
    const PORT: &str = "/dev/ttyUSB0";
    #[cfg(windows)]
    const PORT: &str = "COM1";

    let mut s = SerialPortSettings::default();
    s.baud_rate = 115200;
    s.flow_control = FlowControl::Software;

    let mut port = Serial::from_path(PORT, &s).unwrap();
    #[cfg(unix)]
        port.set_exclusive(false).expect("set_exclusive(false) failed!");

//    let mut port = tokio::fs::File::from_std(std::fs::File::open("/tmp/port").unwrap());

    let (_, reader) = port.framed(Codec).split();
    let printer = reader
        .for_each(|s| {
            println!("received {} bytes: {:?}", s.len(), s);
            Ok(())
        }).map_err(|e| eprintln!("{}", e));

    tokio::run(printer);
}
