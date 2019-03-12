use bytes::Bytes;

use super::Mmap;
use crate::work::Subwork2;

pub struct Writer {
    pub mmap: Mmap,
}

pub struct Reader {
    pub mmap: Mmap,
}

pub enum SerialMode {
    Direct,
    Mining,
}

impl Writer {
    pub fn writer_subwork2(&mut self, sw2: Subwork2) {
        self.mmap.write(0, sw2.version.to_ne_bytes());
        self.mmap.write(4, sw2.vermask.to_ne_bytes());
        assert_eq!(sw2.prevhash.len(), 32);
        self.mmap.write(8, sw2.prevhash);
        assert_eq!(sw2.merkle_root.len(), 32);
        self.mmap.write(40, sw2.merkle_root);
        assert_eq!(sw2.ntime.len(), 4);
        self.mmap.write(72, sw2.ntime);
        assert_eq!(sw2.nbits.len(), 4);
        self.mmap.write(76, sw2.nbits);
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
            SerialMode::Mining => self.set_csr(0, true),
        }
    }
}

impl Reader {
    pub fn read_nonce(&mut self) -> Bytes {
        let mut nonce = Bytes::with_capacity(7);
        nonce.extend(self.mmap.read(0, 7));
        nonce
    }
}
