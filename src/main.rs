#![feature(fnbox)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::prelude::*;
use tokio::timer::Delay;

mod util;
pub mod work;
pub mod stratum;

use self::work::*;
use self::stratum::*;
use self::util::serial::serial_framed;

fn main() {
    let mut pool = Pool::new("cn.ss.btc.com:1800");

    let exts = vec!["minimum-difficulty".to_string(), "version-rolling".to_string()];
    let ext_params = serde_json::json!({
            "version-rolling.mask": "1fffe000",
            "version-rolling.min-bit-count": 2
        });

    // mining.configure
    let ret = pool.configure(exts, ext_params);
    println!("1,{:?}", ret);
    let ret = pool.try_read();
    println!("1.5,{:?}", ret);

    let ret = pool.subscribe();
    println!("2,{:?}", ret);
    let ret = pool.try_read();
    println!("3,{}", ret);
    let ret = pool.authorize("h723n8m.002", "");
    println!("4,{:?}", ret);

    let ws = WorkStream(pool.works.clone());
    let xnonce = pool.xnonce.clone();
    let (sink, stream) = serial_framed("/dev/ttyUSB0").split();
    let sink = Arc::new(Mutex::new(sink));

    let task = {
        let receive_from_asic = stream
            .for_each(|s| {
                println!("received {} bytes: {:?}", s.len(), s);
                Ok(())
            }).map_err(|e| eprintln!("{}", e));

        let send_to_asic = ws
            .for_each(move |w| {
                let xnonce = xnonce.lock().unwrap();
                let sink = sink.clone();

                let send_subwork = SubWorkMaker::new(w, &xnonce)
                    .for_each(move |sw| {
                        let sink = sink.clone();
                        Delay::new(Instant::now() + Duration::from_millis(100))
                            .and_then(move |_| {
                                let mut sink = sink.lock().unwrap();
                                sink.start_send(sw).unwrap();
                                sink.poll_complete().unwrap();
                                Ok(())
                            })
                            .map_err(failure::Error::from)
                    })
                    .map_err(|e| eprintln!("{}", e));
                send_subwork
            })
            .map_err(|_| ());

        receive_from_asic.join(send_to_asic).then(|_| Ok(()))
    };

    tokio::run(task);
}
