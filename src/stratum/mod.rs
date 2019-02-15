use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use futures::stream::Stream;
use futures::task::Task;
use futures::{Async, Future, Poll};
use num_bigint::BigUint;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::reactor::Handle;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio_codec::{Decoder, LinesCodec};

mod message;
mod reader;

pub use self::message::*;
pub use self::reader::Reader;
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
    pub counter: Arc<Mutex<u32>>,
    reader: Option<Receiver<String>>,
    writer: Option<Sender<String>>,
    pub xnonce: Arc<Mutex<(Bytes, usize)>>,
    pub works: Arc<Mutex<(WorkDeque, Option<Task>)>>,
    pub vermask: Arc<Mutex<Option<Bytes>>>,
    pub diff: Arc<Mutex<BigUint>>,
}

impl Pool {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: String::from(addr),
            counter: Arc::new(Mutex::new(0)),
            reader: None,
            writer: None,
            xnonce: Arc::new(Mutex::new((Bytes::new(), 0))),
            works: Arc::new(Mutex::new((WorkDeque::default(), None))),
            vermask: Arc::new(Mutex::new(None)),
            diff: Arc::new(Mutex::new(BigUint::default())),
        }
    }

    fn counter(&mut self) -> Option<u32> {
        let mut counter = self.counter.lock().unwrap();
        *counter += 1;
        Some(*counter)
    }

    pub fn connect(&mut self) -> impl Future<Item = (), Error = ()> + Send + 'static {
        let (reader_tx, reader_rx) = channel::<String>(4096);
        self.reader = Some(reader_rx);

        let (writer_tx, writer_rx) = channel::<String>(4096);
        self.writer = Some(writer_tx);

        let tcpstream = std::net::TcpStream::connect(&self.addr).unwrap();
        let tcpstream = TcpStream::from_std(tcpstream, &Handle::default()).unwrap();
        let (sink, stream) = LinesCodec::new().framed(tcpstream).split();

        let reader = stream
            .for_each(move |line| {
                dbg!(&line);
                let send = reader_tx
                    .clone()
                    .send(line)
                    .and_then(|_| Ok(()))
                    .map_err(|_| ());
                tokio::spawn(send);
                Ok(())
            })
            .map_err(|_| ());

        let writer = writer_rx
            .map_err(|_| std::io::Error::from_raw_os_error(-1))
            .forward(sink)
            .map_err(|_| ());

        reader.join(writer).then(|_| Ok(()))
    }

    pub fn sender(&mut self) -> Sender<String> {
        self.writer.clone().unwrap()
    }

    pub fn receiver(&mut self) -> Receiver<String> {
        self.reader.take().unwrap()
    }

    pub fn send<T: serde::Serialize>(&mut self, msg: T) -> impl Future {
        let mut data = serde_json::to_string(&msg).unwrap();
        data.push('\n');
        self.sender().send(data)
    }

    pub fn subscribe(&mut self) {
        let msg = Action {
            id: self.counter(),
            method: String::from("mining.subscribe"),
            params: Params::None(vec![]),
        };
        let _ = self.send(&msg).wait();
    }

    pub fn authorize(&mut self, user: &str, pass: &str) {
        let msg = Action {
            id: self.counter(),
            method: String::from("mining.authorize"),
            params: Params::User([String::from(user), String::from(pass)]),
        };
        let _ = self.send(&msg).wait();
    }
}
