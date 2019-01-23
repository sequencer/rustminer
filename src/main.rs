#![feature(fnbox)]


use serde_json::json;
use tokio::prelude::*;

mod util;
pub mod work;
pub mod stratum;

use self::work::*;
use self::stratum::*;
use self::util::serial::serial_framed;

fn main() {
    let mut pool = Pool::new("cn.ss.btc.com:1800");

    let exts = vec!["minimum-difficulty".to_string(), "version-rolling".to_string()];
    let ext_params = json!({
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
    let task = ws
        .for_each(move |w| {
            let xnonce = xnonce.lock().unwrap();
            let (sink, stream) = serial_framed("/dev/ttyUSB0").split();
            let send_to_asic = SubWorkMaker::new(w, &xnonce)
                .forward(sink)
                .then(|_| Ok(()));
            tokio::spawn(send_to_asic);

            let receive_from_asic = stream
                .for_each(|s| {
                    println!("received {} bytes: {:?}", s.len(), s);
                    Ok(())
                }).map_err(|e| eprintln!("{}", e));
            tokio::spawn(receive_from_asic);

            Ok(())
        });

    tokio::run(task);
}
