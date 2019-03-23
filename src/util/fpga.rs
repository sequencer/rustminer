use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use bytes::Bytes;
use crc_all::CrcAlgo;
use futures::future::Future;
use futures::sink::Sink;
use lazy_static::lazy_static;
use tokio::sync::mpsc::{channel, Receiver};

use super::Mmap;
use crate::work::Subwork2;

pub struct Writer {
    mmap: Mmap,
    subworks: VecDeque<Subwork2>,
}

pub struct Reader {
    mmap: Arc<Mutex<Mmap>>,
}

pub enum SerialMode {
    Direct,
    Mining,
}

pub fn new() -> (Writer, Reader) {
    let mmap = Mmap::new("/dev/uio0", 100, 0);
    let writer = Writer {
        mmap: mmap.reduce(82),
        subworks: VecDeque::with_capacity(2),
    };
    let reader = Reader {
        mmap: Arc::new(Mutex::new(mmap.offset(84))),
    };
    (writer, reader)
}

pub fn crc5_false(data: &[u8], offset: usize) -> u8 {
    assert!(offset < 8);
    lazy_static! {
        static ref CRC5: CrcAlgo<u8> = CrcAlgo::<u8>::new(0x05, 5, 0x1f, 0, false);
    };
    let crc = &mut 0u8;
    CRC5.init_crc(crc);

    if offset == 0 {
        CRC5.update_crc(crc, data)
    } else {
        CRC5.update_crc(crc, &data[..data.len() - 1]);
        *crc ^= data.last().unwrap() & (0xff << offset);
        for _ in offset..8 {
            if crc.leading_zeros() == 0 {
                *crc = *crc << 1 ^ 0x28;
            } else {
                *crc <<= 1;
            }
        }
        CRC5.finish_crc(crc)
    }
}

pub fn version_bits(mut version_mask: u32, mut version_count: u32) -> u32 {
    version_mask = version_mask.swap_bytes();
    let mut version_bits = 0;

    let mut num = 0;
    while version_mask != 0 {
        let trailing_zeros = version_mask.trailing_zeros();
        num += if trailing_zeros > 0 {
            version_mask >>= trailing_zeros;
            trailing_zeros
        } else {
            let trailing_ones = (!version_mask).trailing_zeros();
            let mask = 0xffff_ffff >> (32 - trailing_ones);
            version_bits |= (version_count & mask) << num;
            version_count >>= trailing_ones;
            version_mask >>= trailing_ones;
            trailing_ones
        };
    }
    version_bits.swap_bytes()
}

impl Writer {
    pub fn writer_subwork2(&mut self, sw2: Subwork2) {
        self.mmap.write(0, sw2.version.to_be_bytes());
        self.mmap.write(4, sw2.vermask.to_be_bytes());
        assert_eq!(sw2.prevhash.len(), 32);
        self.mmap.write(8, &sw2.prevhash);
        assert_eq!(sw2.merkle_root.len(), 32);
        self.mmap.write(40, &sw2.merkle_root);
        assert_eq!(sw2.ntime.len(), 4);
        self.mmap.write(72, &sw2.ntime);
        assert_eq!(sw2.nbits.len(), 4);
        self.mmap.write(76, &sw2.nbits);

        self.subworks.push_front(sw2);
        self.subworks.truncate(2);

        // debug
        print!("write work: ");
        for b in self.mmap.read(0, 80) {
            print!("{:02x}", b);
        }
        println!();
    }

    pub fn subworks(&self) -> Vec<Subwork2> {
        self.subworks.iter().cloned().collect()
    }

    fn set_csr(&mut self, csr: usize, value: bool) {
        assert!(csr < 16);
        let ptr = unsafe { self.mmap.ptr().add(80) };
        let data = unsafe { ptr.read_volatile() };
        let value = if value {
            data | 1 << csr
        } else {
            data & (0xff ^ 1 << csr)
        };

        if data != value {
            unsafe {
                ptr.write_volatile(value);
            }
        }
    }

    pub fn set_serial_mode(&mut self, mode: SerialMode) {
        match mode {
            SerialMode::Direct => self.set_csr(0, false),
            SerialMode::Mining => {
                self.set_csr(0, true);
                self.set_csr(2, true);
            }
        }
    }
}

impl Reader {
    pub fn receive_nonce(&mut self) -> Receiver<Bytes> {
        let (sender, receiver) = channel(32);
        let mmap = self.mmap.clone();

        const ENABLE_INTERRUPT: [u8; 4] = 1u32.to_ne_bytes();

        let reader = move || {
            let mut uio = OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/uio0")
                .expect("can't open /dev/uio0 !");
            let mut buf = [0; 4];
            uio.write_all(&ENABLE_INTERRUPT).unwrap();

            while uio.read(&mut buf).unwrap() == 4 {
                let mut nonce = Bytes::with_capacity(12);
                nonce.extend(mmap.lock().unwrap().read(0, 12));

                sender.clone().send(nonce).wait().unwrap();
                uio.write_all(&ENABLE_INTERRUPT).unwrap();
            }
        };
        thread::spawn(reader);

        receiver
    }
}
