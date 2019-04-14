use crate::util::{hex_to, ToHex};

use super::*;

impl Pool {
    pub(super) fn reader(&mut self) -> impl Future<Item = (), Error = ()> + Send {
        let xnonce = self.xnonce.clone();
        let work_sender = self.work_channel.0.clone();
        let work_notify = self.work_notify.clone();
        let vermask = self.vermask.clone();
        let diff = self.diff.clone();
        self.receiver().for_each(move |line| {
            if let Ok(s) = serde_json::from_str::<Action>(&line) {
                match s.params {
                    Params::Work(w) => {
                        info!("=> received new work!");
                        let work_notify = work_notify.clone();
                        tokio::spawn(work_sender.clone().send(w).then(move |_| {
                            work_notify.notify();
                            Ok(())
                        }));
                    }
                    Params::Num([n]) => {
                        if s.method.as_str() == "mining.set_difficulty" {
                            info!("=> set difficulty: {}!", &n);
                            *diff.lock().unwrap() = n;
                        }
                    }
                    Params::TMask(tmask) => {
                        if s.method.as_str() == "mining.set_version_mask" {
                            let mask = tmask.0[0];
                            info!("=> set vermask: 0x{}!", mask.to_be_bytes().to_hex());
                            *vermask.lock().unwrap() = Some(mask);
                        }
                    }
                    _ => info!("=> {}: {:?}", s.method, s.params),
                }
            } else if let Ok(s) = serde_json::from_str::<Respond>(&line) {
                match s.result {
                    ResultOf::Authorize(r) => {
                        let (action, result);
                        match s.id {
                            Some(2) => {
                                action = "authorized";
                                result = ["successfully", "failed"];
                            }
                            Some(4) => {
                                action = "submitted nonce";
                                result = ["accepted", "rejected"];
                            }
                            _ => {
                                action = "unknown";
                                result = ["true", "false"];
                            }
                        }

                        match r {
                            BoolOrNull::Bool(x) if x => info!("=> {} {}!", action, result[0]),
                            _ => info!("=> {} {}!", action, result[1]),
                        }
                    }
                    ResultOf::Subscribe(r) => {
                        info!("=> set xnonce1: 0x{}, xnonce2_size: {}!", r.1.to_hex(), r.2);
                        let mut xnonce = xnonce.lock().unwrap();
                        xnonce.0 = r.1;
                        xnonce.1 = r.2;
                    }
                    ResultOf::Configure(r) => {
                        if let Some(result) = r.get("version-rolling") {
                            if let serde_json::Value::Bool(result) = result {
                                if *result {
                                    let mask = hex_to::u32(r.get("version-rolling.mask").unwrap())
                                        .unwrap();
                                    info!("=> set vermask: 0x{}!", mask.to_be_bytes().to_hex());
                                    *vermask.lock().unwrap() = Some(mask);
                                } else {
                                    info!("=> the pool does not support version-rolling!");
                                }
                            } else if let serde_json::Value::String(e) = result {
                                info!("=> the pool does not support version-rolling: {:?}!", e);
                            }
                        }
                    }
                }
            }
            Ok(())
        })
    }
}
