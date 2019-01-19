#![feature(fnbox)]

use std::thread;
use std::time::Duration;

use serde_json::json;

mod util;
pub mod work;
pub mod stratum;

use self::work::*;
use self::stratum::*;

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

    loop {
        if let Ok(mut works) = pool.works.clone().lock() {
            if let Some(work) = works.pop() {
                if let Ok(xnonce) = pool.xnonce.lock() {
                    let subworkmaker = SubWorkMaker::new(work, &xnonce);
                    for sw in subworkmaker {
                        println!("{:?}", sw);
                    }
                }
            } else {
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
