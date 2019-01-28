use std::collections::VecDeque;
use std::io;
use std::net::TcpStream;
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use bytes::Bytes;
use failure::Error;
use futures::stream::Stream;
use futures::task::Task;
use futures::{Async, Poll};

mod message;
mod reader;
#[cfg(test)]
mod tests;
mod writer;

pub use self::message::*;
use self::reader::Reader;
use self::writer::Writer;
use super::work::*;

pub type Result<T> = std::result::Result<T, failure::Error>;

#[derive(Debug, Default)]
pub struct WorkDeque(VecDeque<Work>);

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
pub struct WorkStream(pub Arc<Mutex<(WorkDeque, Option<Task>)>>);

impl Stream for WorkStream {
    type Item = Work;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut works = self.0.lock().unwrap();
        match works.0.pop_front() {
            Some(w) => Ok(Async::Ready(Some(w))),
            None => {
                works.1 = Some(futures::task::current());
                Ok(Async::NotReady)
            }
        }
    }
}

pub struct Pool {
    addr: String,
    stream: Option<TcpStream>,
    pub counter: Arc<Mutex<u32>>,
    reader: Option<Reader>,
    writer: Option<Writer>,
    pub xnonce: Arc<Mutex<(Bytes, usize)>>,
    pub works: Arc<Mutex<(WorkDeque, Option<Task>)>>,
    pub vermask: Arc<Mutex<Option<Bytes>>>,
}

impl Pool {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: String::from(addr),
            stream: None,
            counter: Arc::new(Mutex::new(0)),
            reader: None,
            writer: None,
            xnonce: Arc::new(Mutex::new((Bytes::new(), 0))),
            works: Arc::new(Mutex::new((WorkDeque::default(), None))),
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
        let mut counter = self.counter.lock().unwrap();
        *counter += 1;
        Some(*counter)
    }

    pub fn try_connect(&mut self) -> io::Result<&TcpStream> {
        match self.stream {
            Some(ref s)
                if match s.take_error() {
                    Ok(None) => true,
                    Ok(Some(e)) | Err(e) => {
                        println!("{:?}", e);
                        false
                    }
                } =>
            {
                Ok(s)
            }
            _ => {
                self.stream = Some(TcpStream::connect(&self.addr)?);
                Ok(self.stream.as_ref().unwrap())
            }
        }
    }

    pub fn sender(&mut self) -> Arc<Mutex<Sender<String>>> {
        match self.writer {
            Some(ref writer) => writer.sender.clone(),
            None => {
                Writer::spawn(self);
                self.writer.as_ref().unwrap().sender.clone()
            }
        }
    }

    pub fn receiver(&mut self) -> &Receiver<String> {
        match self.reader {
            Some(ref reader) => &reader.receiver,
            None => {
                Reader::spawn(self);
                &self.reader.as_ref().unwrap().receiver
            }
        }
    }

    pub fn try_send<T: serde::Serialize>(&mut self, msg: T) -> Result<()> {
        let mut data = serde_json::to_string(&msg).unwrap();
        data.push('\n');
        self.sender()
            .lock()
            .unwrap()
            .send(data)
            .map_err(Error::from)
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

    pub fn submit(&mut self, params: Params) -> Result<()> {
        let msg = Action {
            id: self.counter(),
            method: String::from("mining.submit"),
            params,
        };
        self.try_send(&msg)
    }
}
