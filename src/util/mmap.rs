use std::fs::OpenOptions;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::ptr::null_mut;

use libc::{off_t, size_t};

#[derive(Debug)]
pub struct Mmap {
    ptr: *mut u8,
    size: size_t,
}

pub struct ReadMmap<'a, T> {
    mmap: &'a mut Mmap,
    offset: usize,
    size: usize,
    counter: usize,
    phantom: PhantomData<T>,
}

impl Mmap {
    pub const unsafe fn uninit() -> Self {
        Self {
            ptr: null_mut(),
            size: 0,
        }
    }

    pub fn new<T: AsRef<Path>>(path: T, offset: off_t, size: size_t) -> Self {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap_or_else(|_| panic!("can't open {}!", path.as_ref().display()));

        let ptr = unsafe {
            libc::mmap(
                null_mut(),
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
        debug_assert!(offset + data.len() <= self.size);
        for (i, v) in data.iter().enumerate() {
            unsafe { self.ptr.add(offset + i).write_volatile(*v) }
        }
    }

    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn write_u32(&mut self, offset: usize, value: u32) {
        debug_assert_eq!((self.ptr as usize + offset) & 0b11, 0);
        debug_assert!(offset + 4 <= self.size);
        (self.ptr.add(offset) as *mut u32).write_volatile(value)
    }

    pub unsafe fn write_as_u32<T: AsRef<[u8]>>(&mut self, mut offset: usize, data: T) {
        let data = data.as_ref();
        let len = data.len();
        debug_assert_eq!((self.ptr as usize + offset) & 0b11, 0);
        debug_assert_eq!(len & 0b11, 0);
        debug_assert!(len >= 4 && offset + len <= self.size);
        for v in data.iter().step_by(4) {
            self.write_u32(
                offset,
                u32::from_ne_bytes(*((v as *const u8) as *const [u8; 4])),
            );
            offset += 4;
        }
    }

    pub fn read(&mut self, offset: usize, size: usize) -> ReadMmap<u8> {
        ReadMmap::<u8>::new(self, offset, size)
    }

    pub unsafe fn read_u32(&mut self, offset: usize, size: usize) -> ReadMmap<u32> {
        ReadMmap::<u32>::new(self, offset, size)
    }

    pub unsafe fn from_raw(ptr: *mut u8, size: usize) -> Self {
        Self { ptr, size }
    }

    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn size(&self) -> size_t {
        self.size
    }
}

impl<'a> ReadMmap<'a, u8> {
    pub fn new(mmap: &'a mut Mmap, offset: usize, size: usize) -> Self {
        Self {
            mmap,
            offset,
            size,
            counter: 0,
            phantom: PhantomData,
        }
    }
}

impl<'a> Iterator for ReadMmap<'a, u8> {
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

impl<'a> ReadMmap<'a, u32> {
    pub unsafe fn new(mmap: &'a mut Mmap, offset: usize, size: usize) -> Self {
        debug_assert_eq!((mmap.ptr as usize + offset) & 0b11, 0);
        debug_assert_eq!(size & 0b11, 0);
        Self {
            mmap,
            offset,
            size,
            counter: 0,
            phantom: PhantomData,
        }
    }
}

impl<'a> Iterator for ReadMmap<'a, u32> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter >= self.size {
            None
        } else {
            let value = unsafe {
                #[allow(clippy::cast_ptr_alignment)]
                (self.mmap.ptr.add(self.offset + self.counter) as *mut u32).read_volatile()
            };
            self.counter += 4;
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

unsafe impl Sync for Mmap {}
