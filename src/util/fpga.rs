use std::collections::VecDeque;
use std::io::Error;
use std::sync::Once;
use std::thread;
use std::time::Duration;

use bytes::Bytes;
use crc_all::CrcAlgo;
use futures::sync::mpsc::{channel, Receiver};
use futures::{Async, Future, Poll, Sink, Stream};
use lazy_static::lazy_static;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::runtime::current_thread;
use tokio_uio::Uio;

use super::Mmap;
use crate::util::hex::ToHex;
use crate::work::Subwork2;

static mut UIO_MMAP: Mmap = unsafe { Mmap::uninitialized() };

pub fn mmap(offset: usize, size: usize) -> Mmap {
    static INIT: Once = Once::new();

    let uio_mmap = unsafe {
        INIT.call_once(|| {
            UIO_MMAP = Mmap::new("/dev/uio0", 0, 161);
        });
        &UIO_MMAP
    };
    assert!(offset + size <= uio_mmap.size());

    unsafe { Mmap::from_raw(uio_mmap.ptr().add(offset), size) }
}

pub struct Csr {
    mmap: Mmap,
}

pub struct Writer {
    data: Mmap,
    io_select: Csr,
    io_enable: Csr,
    subworks: VecDeque<Subwork2>,
}

pub struct Reader {
    data: Option<Mmap>,
    csr_in: Option<Csr>,
}

pub struct SerialSender {
    data: Mmap,
    csr_in: Csr,
    csr_out: Csr,
    io_select: Csr,
    io_enable: Csr,
}

pub fn writer() -> Writer {
    Writer {
        data: mmap(0, 80),
        io_select: Csr::new(mmap(84, 1)),
        io_enable: Csr::new(mmap(85, 1)),
        subworks: VecDeque::with_capacity(2),
    }
}

pub fn reader() -> Reader {
    Reader {
        data: Some(mmap(148, 13)),
        csr_in: Some(Csr::new(mmap(80, 1))),
    }
}

pub fn serial_sender() -> SerialSender {
    SerialSender {
        data: mmap(86, 62),
        csr_in: Csr::new(mmap(80, 1)),
        csr_out: Csr::new(mmap(82, 1)),
        io_select: Csr::new(mmap(84, 1)),
        io_enable: Csr::new(mmap(85, 1)),
    }
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

impl Csr {
    fn new(mmap: Mmap) -> Self {
        Self { mmap }
    }

    pub fn set_csr(&mut self, csr: usize, value: bool) {
        assert!(csr < 8);
        let ptr = self.mmap.ptr();
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

    pub fn get_csr(&mut self, csr: usize) -> bool {
        assert!(csr < 8);

        let data = unsafe { self.mmap.ptr().read_volatile() };
        let value = 1 << csr;
        data & value == value
    }

    pub fn set_all(&mut self, value: bool) {
        unsafe {
            if value {
                self.mmap.ptr().write_volatile(0xff);
            } else {
                self.mmap.ptr().write_volatile(0);
            }
        }
    }

    pub fn notify(&mut self, csr: usize) {
        assert!(csr < 8);
        self.set_csr(csr, true);
        self.set_csr(csr, false);
    }
}

impl Writer {
    pub fn writer_subwork2(&mut self, sw2: Subwork2) {
        self.data.write(0, sw2.version.to_be_bytes());
        self.data.write(4, sw2.vermask.to_be_bytes());
        debug_assert_eq!(sw2.prevhash.len(), 32);
        self.data.write(8, &sw2.prevhash);
        debug_assert_eq!(sw2.merkle_root.len(), 32);
        self.data.write(40, &sw2.merkle_root);
        debug_assert_eq!(sw2.ntime.len(), 4);
        self.data.write(72, &sw2.ntime);
        debug_assert_eq!(sw2.nbits.len(), 4);
        self.data.write(76, &sw2.nbits);

        self.subworks.push_front(sw2);
        self.subworks.truncate(2);

        // debug
        debug!(
            "written work: {}",
            self.data.read(0, 80).collect::<Vec<u8>>().to_hex()
        );
    }

    pub fn enable_sender(&mut self, board: usize) {
        self.io_select.set_all(false);
        self.io_select.set_csr(board, false);
        self.io_enable.set_csr(board, true);
    }

    pub fn subworks(&self) -> Vec<Subwork2> {
        self.subworks.iter().cloned().collect()
    }
}

struct UioReader {
    inner: Uio,
}

struct UioWriter<'a> {
    inner: &'a mut Uio,
    buf: &'a [u8],
}

impl<'a> Future for UioWriter<'a> {
    type Item = usize;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        self.inner.poll_write(self.buf)
    }
}

impl Default for UioReader {
    fn default() -> Self {
        Self {
            inner: Uio::open("/dev/uio0").expect("can't open /dev/uio0!"),
        }
    }
}

impl UioReader {
    fn enable(&mut self) -> UioWriter {
        const ENABLE_INTERRUPT: [u8; 4] = 1u32.to_ne_bytes();
        UioWriter {
            inner: &mut self.inner,
            buf: &ENABLE_INTERRUPT,
        }
    }
}

impl Stream for UioReader {
    type Item = usize;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.enable().wait().unwrap();
        self.inner
            .poll_read(&mut [0; 4])
            .map(|x| x.map(Some))
            .map_err(|_| ())
    }
}

impl Reader {
    pub fn receive_nonce(&mut self) -> Receiver<Bytes> {
        let (sender, receiver) = channel(32);
        let mut mmap = self.data.take().unwrap();
        let mut csr_in = self.csr_in.take().unwrap();

        let reader = move || {
            let read_nonce = UioReader::default().for_each(move |x| {
                if x == 4 {
                    let mut nonce = Bytes::with_capacity(12);
                    nonce.extend(mmap.read(0, 12));

                    sender.clone().send(nonce).wait().unwrap();
                    csr_in.notify(3);
                }
                Ok(())
            });

            let mut runtime = current_thread::Runtime::new().unwrap();
            let _ = runtime.block_on(read_nonce);
        };
        thread::spawn(reader);

        receiver
    }
}

impl SerialSender {
    pub fn select_board(&mut self, board: usize) {
        self.io_select.set_all(false);
        self.io_select.set_csr(board, true);
        self.io_enable.set_csr(board, false);
    }

    pub fn enable_sender(&mut self, board: usize) {
        self.io_select.set_all(false);
        self.io_select.set_csr(board, false);
        self.io_enable.set_csr(board, true);
    }

    pub fn set_direct(&mut self) {
        self.csr_in.set_csr(0, false);
    }

    pub fn set_send_work(&mut self) {
        self.csr_in.set_csr(0, true);
    }

    pub fn unselect_all(&mut self) {
        self.io_select.set_all(false);
    }

    pub fn writer_work(&mut self, work: &[u8], interval: u16) {
        assert!(work.len() <= 56);
        self.set_send_work();

        loop {
            if self.csr_out.get_csr(0) {
                // set interval
                self.data.write(0, interval.to_le_bytes());
                self.data.write(2, work);
                self.csr_in.notify(1);
                break;
            } else {
                thread::sleep(Duration::from_micros(10))
            }
        }
    }

    pub fn get_count(&mut self) -> u32 {
        let mut count = Vec::with_capacity(4);
        count.extend(self.data.read(58, 4));
        u32::from_le_bytes(unsafe { *(count.as_ptr() as *const [u8; 4]) })
    }
}
