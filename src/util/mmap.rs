use std::fs::OpenOptions;
use std::ops::{Index, IndexMut, Range};
use std::os::unix::io::AsRawFd;
use std::path::Path;

use libc::{off_t, size_t};

#[derive(Debug)]
pub struct Mmap {
    ptr: *mut u8,
    size: size_t,
}

pub struct ReadMmap<'a> {
    mmap: &'a mut Mmap,
    offset: usize,
    size: usize,
    counter: usize,
}

impl Mmap {
    pub fn new<T: AsRef<Path>>(path: T, size: size_t, offset: off_t) -> Self {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap_or_else(|_| panic!("can't open {} !", path.as_ref().display()));

        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                f.as_raw_fd(),
                offset,
            )
        } as *mut u8;
        Self { size, ptr }
    }

    pub fn write<T: AsRef<[u8]>>(&mut self, offset: usize, data: T) {
        let data = data.as_ref();
        assert!(offset + data.len() <= self.size());
        for (i, v) in data.iter().enumerate() {
            unsafe { self.ptr.add(offset + i).write_volatile(*v) }
        }
    }

    pub fn read(&mut self, offset: usize, size: usize) -> ReadMmap {
        ReadMmap::new(self, offset, size)
    }

    pub fn offset(&self, offset: usize) -> Self {
        assert!(offset < self.size);
        Self {
            ptr: unsafe { self.ptr.add(offset) },
            size: self.size - offset,
        }
    }

    pub fn reduce(&self, size: usize) -> Self {
        assert!(size < self.size);
        Self {
            ptr: self.ptr,
            size,
        }
    }

    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn size(&self) -> size_t {
        self.size
    }
}

impl<'a> ReadMmap<'a> {
    pub fn new(mmap: &'a mut Mmap, offset: usize, size: usize) -> Self {
        Self {
            mmap,
            offset,
            size,
            counter: 0,
        }
    }
}

impl<'a> Iterator for ReadMmap<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter >= self.size {
            None
        } else {
            let value = unsafe {
                self.mmap
                    .ptr
                    .add(self.offset + self.counter)
                    .read_volatile()
            };
            self.counter += 1;
            Some(value)
        }
    }
}

impl Index<usize> for Mmap {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.size);
        unsafe { &*self.ptr.add(index) }
    }
}

impl Index<Range<usize>> for Mmap {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        assert!(index.start <= index.end);
        assert!(index.end < self.size);
        let ptr = unsafe { self.ptr.add(index.start) };
        let len = index.end - index.start;
        unsafe { core::slice::from_raw_parts(ptr, len) }
    }
}

impl IndexMut<usize> for Mmap {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.size);
        unsafe { &mut *self.ptr.add(index) }
    }
}

impl IndexMut<Range<usize>> for Mmap {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        assert!(index.start <= index.end);
        assert!(index.end < self.size);
        let ptr = unsafe { self.ptr.add(index.start) };
        let len = index.end - index.start;
        unsafe { core::slice::from_raw_parts_mut(ptr, len) }
    }
}

unsafe impl Send for Mmap {}
