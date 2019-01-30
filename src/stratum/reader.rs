use std::io::{BufRead, BufReader};

use super::super::util::hex_to;
use super::*;

pub struct Reader {
    pub receiver: Receiver<String>,
    handle: JoinHandle<()>,
}

impl Reader {
    pub fn spawn(pool: &mut Pool) {
        let stream = pool.try_connect().unwrap().try_clone().unwrap();
        let xnonce = pool.xnonce.clone();
        let works = pool.works.clone();
        let vermask = pool.vermask.clone();
        let diff = pool.diff.clone();
        let mut bufr = BufReader::new(stream);
        let (data_tx, data_rx) = mpsc::channel();
        let handle = thread::spawn(move || loop {
            let mut buf = String::new();
            if bufr.read_line(&mut buf).is_ok() {
                if let Ok(s) = serde_json::from_str::<Action>(&buf) {
                    dbg!(&buf);
                    match s.params {
                        Params::Work(w) => {
                            let mut works = works.lock().unwrap();
                            if w.clean {
                                works.0.clear();
                            }
                            works.0.push_back(w);
                            match &works.1 {
                                Some(t) => t.notify(),
                                None => (),
                            }
                            println!("received new work!");
                        }
                        Params::Integer([n]) => {
                            if s.method.as_str() == "mining.set_difficulty" {
                                let mut diff = diff.lock().unwrap();
                                *diff = BigUint::from(n);
                                println!("set difficulty: {}!", n);
                            }
                        }
                        _ => println!("=> {}: {:?}", s.method, s.params),
                    }
                } else if let Ok(s) = serde_json::from_str::<Respond>(&buf) {
                    match s.result {
                        ResultOf::Authorize(r) => {
                            if r {
                                println!("authorized successfully!");
                            } else {
                                println!("authorized failed!");
                            }
                        }
                        ResultOf::Subscribe(r) => {
                            let mut xnonce = xnonce.lock().unwrap();
                            xnonce.0 = r.1;
                            xnonce.1 = r.2;
                            println!(
                                "=> set xnonce1: {:?}, xnonce2_size: {}!",
                                xnonce.0, xnonce.1
                            );
                        }
                        ResultOf::Configure(r) => {
                            if let Some(result) = r.get("version-rolling") {
                                if let serde_json::Value::Bool(result) = result {
                                    if *result {
                                        let mut vermask = vermask.lock().unwrap();
                                        let mask =
                                            hex_to::bytes(r.get("version-rolling.mask").unwrap())
                                                .unwrap();
                                        *vermask = Some(mask);
                                        println!("=> set vermask: {:?}!", *vermask);
                                    } else {
                                        println!("the pool does not support version-rolling!");
                                    }
                                } else if let serde_json::Value::String(e) = result {
                                    println!("the pool does not support version-rolling: {:?}!", e);
                                }
                            }
                        }
                    }
                }
            }
            if let Err(e) = data_tx.send(buf) {
                println!("Reader send err: {:?}!", e);
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
