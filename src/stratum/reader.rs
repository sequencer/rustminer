use std::io::{BufReader, BufRead};

use super::*;

pub struct Reader {
    pub receiver: Receiver<String>,
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
