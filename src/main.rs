use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::json;
use tokio::prelude::*;
use tokio::runtime::current_thread;

pub mod stratum;
pub mod util;
pub mod work;

use self::stratum::*;
use self::util::{fpga, ToHex};
use self::work::*;
use crate::util::mmap::Mmap;

fn main_loop() {
    let mut pool = Pool::new("cn.ss.btc.com:1800");

    let connect_pool = pool.connect();
    let reader = Reader::create(&mut pool);
    let checker = checker::new(&mut pool);
    let connect_pool = connect_pool.join3(reader, checker);

    let exts = vec!["minimum-difficulty".to_string(), "version-rolling".to_string()];
    let ext_params = json!({
            "version-rolling.mask": "1fffe000",
            "version-rolling.min-bit-count": 2
        });

    // mining.configure
    pool.configure(exts, ext_params);

    pool.subscribe();
    pool.authorize("h723n8m.001", "");

    let pool_sender = pool.sender();

    let ws = WorkStream(pool.work_channel.1);
    let xnonce = pool.xnonce.clone();
    let vermask = pool.vermask.clone();
    let has_new_work = pool.has_new_work.clone();
    let mut fpga_writer = fpga::Writer{mmap: Mmap::new("/dev/uio0", 80, 0)};

    let send_to_fpga = ws
        .map(move |w| {
            Subwork2Maker::new(
                w,
                &xnonce.lock().unwrap(),
                vermask.lock().unwrap().unwrap(),
                has_new_work.clone(),
            )
        })
        .flatten()
        .for_each(move |sw2| {
            dbg!(&sw2);
            fpga_writer.writer_subwork2(sw2);
            thread::sleep(Duration::from_secs(1));
            Ok(())
        });

    thread::spawn(move || {
        let mut runtime = current_thread::Runtime::new().unwrap();
        runtime.block_on(send_to_fpga).unwrap();
    });

    let mut runtime = current_thread::Runtime::new().unwrap();
    runtime.block_on(connect_pool).unwrap();
}

fn main() {
    loop {
        main_loop();
    }
}
