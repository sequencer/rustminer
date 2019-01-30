#![feature(fnbox)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::prelude::*;
use tokio::timer::Delay;

pub mod stratum;
mod util;
pub mod work;

use self::stratum::*;
use self::util::serial::serial_framed;
use self::work::*;

fn main() {
    let mut pool = Pool::new("cn.ss.btc.com:1800");

//    let exts = vec!["minimum-difficulty".to_string(), "version-rolling".to_string()];
//    let ext_params = serde_json::json!({
//            "version-rolling.mask": "1fffe000",
//            "version-rolling.min-bit-count": 2
//        });
//
//    // mining.configure
//    let ret = pool.configure(exts, ext_params);
//    println!("1,{:?}", ret);
//    let ret = pool.try_read();
//    println!("1.5,{:?}", ret);

    let ret = pool.subscribe();
    println!("2,{:?}", ret);
    let ret = pool.try_read();
    println!("3,{}", ret);
    let ret = pool.authorize("h723n8m.002", "");
    println!("4,{:?}", ret);

    let pool_sender = pool.sender();

    let ws = WorkStream(pool.works.clone());
    let xnonce = pool.xnonce.clone();
    let (sink, stream) = serial_framed("/dev/ttyUSB0").split();
    let sink = Arc::new(Mutex::new(sink));

    let task = {
        let pool_diff = pool.diff.clone();
        let receive_from_asic = stream
            .for_each(move |sw| {
                println!("received: {:?}", sw);
                let diff = sw.0.diff(&sw.1);
                let pool_diff = pool_diff.lock().unwrap();
                if diff >= *pool_diff {
                    let params = sw.0.into_params("h723n8m.002", sw.1);
                    let msg = Action {
                        id: Some(4),
                        method: String::from("mining.submit"),
                        params,
                    };
                    let mut data = msg.to_string().unwrap();
                    data.push('\n');
                    let _ = pool_sender.lock().unwrap().send(data);
                } else {
                    eprintln!(
                        "nonce difficulty: {} is too low, required {}!",
                        diff, *pool_diff
                    );
                }
                Ok(())
            })
            .map_err(|e| eprintln!("{}", e));

        let send_to_asic = ws
            .for_each(move |w| {
                let xnonce = xnonce.lock().unwrap();
                let sink = sink.clone();

                // send_subwork
                SubworkMaker::new(w, &xnonce)
                    .for_each(move |sw| {
                        let sink = sink.clone();
                        // delay_send
                        Delay::new(Instant::now() + Duration::from_millis(100))
                            .and_then(move |_| {
                                let mut sink = sink.lock().unwrap();
                                sink.start_send(sw).unwrap();
                                sink.poll_complete().unwrap();
                                Ok(())
                            })
                            .map_err(failure::Error::from)
                    })
                    .map_err(|e| eprintln!("{}", e))
            })
            .map_err(|_| ());

        receive_from_asic.join(send_to_asic).then(|_| Ok(()))
    };

    tokio::run(task);
}
