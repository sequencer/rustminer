use super::*;
use crate::util::{hex_to, ToHex};

impl Pool {
    pub(super) fn reader(&mut self) -> impl Future<Item = (), Error = ()> + Send {
        let xnonce = self.xnonce.clone();
        let work_sender = self.work_channel.0.clone();
        let work_notify = self.work_notify.clone();
        let vermask = self.vermask.clone();
        let diff = self.diff.clone();
        self.receiver()
            .for_each(move |line| {
                if let Ok(s) = serde_json::from_str::<Action>(&line) {
                    match s.params {
                        Params::Work(w) => {
                            let work_notify = work_notify.clone();
                            tokio::spawn(work_sender.clone().send(w).then(move |_| {
                                work_notify.notify();
                                println!("=> received new work!");
                                Ok(())
                            }));
                        }
                        Params::Num([n]) => {
                            if s.method.as_str() == "mining.set_difficulty" {
                                println!("=> set difficulty: {}!", &n);
                                *diff.lock().unwrap() = n;
                            }
                        }
                        Params::TMask(tmask) => {
                            if s.method.as_str() == "mining.set_version_mask" {
                                let mut vermask = vermask.lock().unwrap();
                                let mask = tmask.0[0];
                                *vermask = Some(mask);
                                println!("=> set vermask: 0x{}!", mask.to_be_bytes().to_hex());
                            }
                        }
                        _ => println!("=> {}: {:?}", s.method, s.params),
                    }
                } else if let Ok(s) = serde_json::from_str::<Respond>(&line) {
                    match s.result {
                        ResultOf::Authorize(r) => {
                            let action = match s.id {
                                Some(2) => "authorized",
                                Some(4) => "submit",
                                _ => "unknown",
                            };

                            if r {
                                println!("=> {} successfully!", action);
                            } else {
                                println!("=> {} failed!", action);
                            }
                        }
                        ResultOf::Subscribe(r) => {
                            let mut xnonce = xnonce.lock().unwrap();
                            xnonce.0 = r.1;
                            xnonce.1 = r.2;
                            println!(
                                "=> set xnonce1: 0x{}, xnonce2_size: {}!",
                                xnonce.0.to_hex(),
                                xnonce.1
                            );
                        }
                        ResultOf::Configure(r) => {
                            if let Some(result) = r.get("version-rolling") {
                                if let serde_json::Value::Bool(result) = result {
                                    if *result {
                                        let mut vermask = vermask.lock().unwrap();
                                        let mask =
                                            hex_to::u32(r.get("version-rolling.mask").unwrap())
                                                .unwrap();
                                        *vermask = Some(mask);
                                        println!(
                                            "=> set vermask: 0x{}!",
                                            mask.to_be_bytes().to_hex()
                                        );
                                    } else {
                                        println!("=> the pool does not support version-rolling!");
                                    }
                                } else if let serde_json::Value::String(e) = result {
                                    println!(
                                        "=> the pool does not support version-rolling: {:?}!",
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
                Ok(())
            })
            .map_err(|_| ())
    }
}
