use std::io::{BufReader, BufRead};

use super::*;

pub struct Reader {
    pub receiver: Receiver<String>,
    handle: JoinHandle<()>,
}

impl Reader {
    pub fn new(pool: &mut Pool) {
        let stream = pool.try_connect().unwrap().try_clone().unwrap();
        let xnonce = pool.xnonce.clone();
        let mut bufr = BufReader::new(stream);
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
                                let mut xnonce = xnonce.lock().unwrap();
                                xnonce.0 = r.1;
                                xnonce.1 = r.2;
                                println!("set xnonce1: {:?}, xnonce2_size: {}!", xnonce.0, xnonce.1);
                            }
                        }
                    }
                }
                if let Err(e) = data_tx.send(buf) {
                    println!("Reader send err: {:?}!", e);
                }
            }
        });
        pool.reader = Some(Self {
            receiver: data_rx,
            handle,
        });
    }

    pub fn join(self) -> thread::Result<()> {
        self.handle.join()
    }
}
