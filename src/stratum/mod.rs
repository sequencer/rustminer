use std::io::prelude::*;
use std::io::{self, BufReader, BufRead};
use std::net::TcpStream;
use std::thread::{self, JoinHandle};
use std::sync::mpsc::{self, Sender, Receiver};
use std::boxed::FnBox;

use failure::{Error, ResultExt};

use super::work::*;
use self::message::*;

type Result<T> = std::result::Result<T, Error>;

mod message;
#[cfg(test)]
mod tests;

#[allow(dead_code)]
struct Writer {
    sender: Sender<String>,
    handle: JoinHandle<()>,
    result: Receiver<Result<usize>>,
}

impl Writer {
    pub fn new(stream: &TcpStream) -> Self {
        let mut stream = stream.try_clone().unwrap();
        let (data_tx, data_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut data = String::new();
            loop {
                let _ = result_tx.send(
                    Box::new(
                        |rx: &Receiver<String>| -> Result<usize> {
                            data = rx.recv().context("Writer recv err!")?;
                            Ok(stream.write(data.as_bytes()).context("TcpSteam write err!")?)
                        }).call_box((&data_rx, ))
                );
            };
        });
        Self {
            sender: data_tx,
            handle,
            result: result_rx,
        }
    }

    pub fn join(self) -> thread::Result<()> {
        self.handle.join()
    }
}

struct Reader {
    receiver: Receiver<String>,
    handle: JoinHandle<()>,
}

impl Reader {
    pub fn new(stream: &TcpStream) -> Self {
        let mut bufr = BufReader::new(stream.try_clone().unwrap());
        let (data_tx, data_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            loop {
                let mut buf = String::new();
                if let Ok(_) = bufr.read_line(&mut buf) {
                    if let Ok(s) = serde_json::from_str::<Action>(&buf) {
                        println!("==> {:?}", s.params);
                    } else if let Ok(s) = serde_json::from_str::<Respond>(&buf) {
                        match s.result {
                            ResultOf::Authorize(r) => if r {
                                println!("authorized successfully!");
                            } else {
                                println!("authorized failed!");
                            },
                            ResultOf::Subscribe(r) => {
                                let xnonce1 = r.1;
                                let xnonce2_size = r.2;
                                println!("xnonce1: {:?}, xnonce2_size: {}", xnonce1, xnonce2_size);
                            }
                        }
                    }
                }
                if let Err(e) = data_tx.send(buf) {
                    println!("Reader send err: {:?}!", e);
                }
            }
        });
        Self {
            receiver: data_rx,
            handle,
        }
    }

    pub fn join(self) -> thread::Result<()> {
        self.handle.join()
    }
}

pub struct Pool {
    addr: String,
    stream: Option<TcpStream>,
    counter: u32,
    reader: Option<Reader>,
    writer: Option<Writer>,
}

impl Pool {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: String::from(addr),
            stream: None,
            counter: 0,
            reader: None,
            writer: None,
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
                self.reader = Some(Reader::new(&self.try_connect().unwrap()));
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
