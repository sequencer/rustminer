use super::super::util::hex_to;
use super::*;

pub struct Reader;

impl Reader {
    pub fn create(pool: &mut Pool) -> impl Future<Item = (), Error = ()> + Send + 'static {
        let xnonce = pool.xnonce.clone();
        let works = pool.works.clone();
        let vermask = pool.vermask.clone();
        let diff = pool.diff.clone();
        pool.receiver()
            .for_each(move |line| {
                if let Ok(s) = serde_json::from_str::<Action>(&line) {
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
                } else if let Ok(s) = serde_json::from_str::<Respond>(&line) {
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
                Ok(())
            })
            .map_err(|_| ())
    }
}
