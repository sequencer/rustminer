use std::io;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};

use bytes::Bytes;
pub use failure::{Error, ResultExt};
use futures::stream::Stream;
use futures::Async;

use super::work::*;

use self::message::*;
use self::reader::Reader;
use self::writer::Writer;

pub type Result<T> = std::result::Result<T, failure::Error>;

mod message;
mod writer;
mod reader;
#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct WorkDeque(VecDeque<Work>);

impl WorkDeque {
    pub fn new() -> Self {
        Self(VecDeque::new())
    }
}

impl Deref for WorkDeque {
    type Target = VecDeque<Work>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WorkDeque {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct WorkDequeStream<'a> {
    pub works: &'a Arc<Mutex<WorkDeque>>
}

impl<'a> Stream for WorkDequeStream<'a> {
    type Item = Work;
    type Error = ();

    fn poll(&mut self) -> std::result::Result<Async<Option<Self::Item>>, Self::Error> {
        dbg!(&self);
        match self.works.lock().unwrap().pop_front() {
            Some(w) => Ok(Async::Ready(Some(w))),
            None => Ok(Async::NotReady)
        }
    }
}

pub struct Pool {
    addr: String,
    stream: Option<TcpStream>,
    counter: u32,
    reader: Option<Reader>,
    writer: Option<Writer>,
    pub xnonce: Arc<Mutex<(Bytes, usize)>>,
    pub works: Arc<Mutex<WorkDeque>>,
    pub vermask: Arc<Mutex<Option<Bytes>>>,
}

impl Pool {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: String::from(addr),
            stream: None,
            counter: 0,
            reader: None,
            writer: None,
            xnonce: Arc::new(Mutex::new((Bytes::new(), 0))),
            works: Arc::new(Mutex::new(WorkDeque::new())),
            vermask: Arc::new(Mutex::new(None)),
        }
    }

    // TODO
    #[allow(unused)]
    pub fn join_all(self) {
        self.reader.unwrap().join();
        self.writer.unwrap().join();
    }

    fn counter(&mut self) -> Option<u32> {
        self.counter = self.counter + 1;
        Some(self.counter)
    }

    pub fn try_connect(&mut self) -> io::Result<&TcpStream> {
        match self.stream {
            Some(ref s) if match s.take_error() {
                Ok(None) => true,
                Ok(Some(e)) | Err(e) => {
                    println!("{:?}", e);
                    false
                }
            } => Ok(s),
            _ => {
                self.stream = Some(TcpStream::connect(&self.addr)?);
                Ok(self.stream.as_ref().unwrap())
            }
        }
    }

    pub fn sender(&mut self) -> &Sender<String> {
        match self.writer {
            Some(ref writer) => &writer.sender,
            None => {
                Writer::new(self);
                &self.writer.as_ref().unwrap().sender
            }
        }
    }

    pub fn receiver(&mut self) -> &Receiver<String> {
        match self.reader {
            Some(ref reader) => &reader.receiver,
            None => {
                Reader::new(self);
                &self.reader.as_ref().unwrap().receiver
            }
        }
    }

    pub fn try_send<T: serde::Serialize>(&mut self, msg: T) -> Result<()> {
        let mut data = serde_json::to_string(&msg).unwrap();
        data.push('\n');
        self.sender().send(data).map_err(Error::from)
    }

    pub fn try_read(&mut self) -> String {
        self.receiver().recv().unwrap()
    }

    pub fn configure(&mut self, exts: Vec<String>, ext_params: serde_json::Value) -> Result<()> {
        let msg = Action {
            id: Some(1),
            method: String::from("mining.configure"),
            params: Params::Config(Config(exts, ext_params)),
        };

        self.try_send(&msg)
    }

    pub fn subscribe(&mut self) -> Result<()> {
        let msg = Action {
            id: self.counter(),
            method: String::from("mining.subscribe"),
            params: Params::None(vec![]),
        };
        self.try_send(&msg)
    }

    pub fn authorize(&mut self, user: &str, pass: &str) -> Result<()> {
        let msg = Action {
            id: self.counter(),
            method: String::from("mining.authorize"),
            params: Params::User([String::from(user), String::from(pass)]),
        };
        self.try_send(&msg)
    }
}
