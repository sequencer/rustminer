use std::io;
use std::net::TcpStream;
use std::thread::{self, JoinHandle};
use std::sync::{Mutex, Arc};
use std::sync::mpsc::{self, Sender, Receiver};

use bytes::Bytes;
pub use failure::{Error, ResultExt};

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

pub struct Pool {
    addr: String,
    stream: Option<TcpStream>,
    counter: u32,
    reader: Option<Reader>,
    writer: Option<Writer>,
    xnonce: Arc<Mutex<(Bytes, usize)>>,
    works: Arc<Mutex<Vec<Work>>>,
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
            works: Arc::new(Mutex::new(Vec::new())),
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
                self.writer = Some(Writer::new(&self.try_connect().unwrap()));
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
