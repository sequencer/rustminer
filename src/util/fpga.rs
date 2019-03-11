use super::mmap::*;
use crate::work::Subwork2;

pub struct Writer {
    pub mmap: Mmap,
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
}
