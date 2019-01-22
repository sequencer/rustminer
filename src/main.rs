#![feature(fnbox)]


use serde_json::json;
use tokio::prelude::*;
use tokio::runtime::current_thread::Runtime;

mod util;
pub mod work;
pub mod stratum;

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

    let wds = WorkDequeStream { works: &pool.works };
    let task = wds
        .for_each(|w| {
            println!("{:?}", w);
            Ok(())
        });
    let mut runtime = Runtime::new().unwrap();

    runtime.block_on(task).unwrap();
}
