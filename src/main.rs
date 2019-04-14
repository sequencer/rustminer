#![feature(const_int_conversion)]
#![allow(clippy::unreadable_literal)]

#[macro_use]
extern crate log;

use std::iter::FromIterator;
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;

use bytes::Bytes;
use serde_json::json;
use tokio::prelude::*;
//use tokio::runtime::current_thread;

use self::stratum::*;
use self::util::{
    fpga,
    i2c::{self, BoardConfig},
    ToHex,
};
use self::work::*;

pub mod stratum;
pub mod util;
pub mod work;

fn main_loop() {
    let mut pool = Pool::new("121.29.19.24:443");
    let connect_pool = pool.connect();
    let checker = pool.checker();

    let exts = vec![String::from("version-rolling")];
    let ext_params = json!({
        "version-rolling.mask": "1fffe000",
        "version-rolling.min-bit-count": 2
    });

    // mining.configure
    pool.configure(exts, ext_params);

    pool.subscribe();
    pool.authorize("h723n8m.001", "");

    let pool_sender = pool.sender();
    let pool_diff = pool.diff.clone();

    let ws = WorkStream(pool.work_channel.1);
    let xnonce = pool.xnonce.clone();
    let vermask = pool.vermask.clone();
    let work_notify = pool.work_notify.clone();

    let mut fpga_writer = fpga::writer();
    fpga_writer.enable_sender(5);
    let fpga_writer = Arc::new(Mutex::new(fpga_writer));

    let fpga_writer_clone = fpga_writer.clone();
    let send_to_fpga = ws.for_each(move |w| {
        let fpga_writer = fpga_writer_clone.clone();
        let work_notify = work_notify.clone();

        Subwork2Maker::new(
            w,
            &xnonce.lock().unwrap(),
            vermask.lock().unwrap().unwrap(),
            work_notify.clone(),
        )
        .for_each(move |sw2| {
            //debug!("{:?}", &sw2);
            fpga_writer.lock().unwrap().writer_subwork2(sw2);

            work_notify
                .clone()
                .timeout(Duration::from_secs(10))
                .then(|_| Ok(()))
        })
    });

    let (nonce_reader, nonce_receiver) = fpga::reader().read_nonce();

    //thread::spawn(|| {
    //    let mut runtime = current_thread::Runtime::new().unwrap();
    //    let _ = runtime.block_on(nonce_reader);
    //});

    let mut offset = 0;
    let receive_nonce = nonce_receiver.for_each(move |received| {
        let fpga_writer = fpga_writer.clone();
        let nonce = Bytes::from_iter(received[0..4].iter().rev().cloned());
        let version_count =
            u32::from_le_bytes(unsafe { *(received[8..12].as_ptr() as *const [u8; 4]) })
                - u32::from((received[7] - received[5]) & 0x7f);

        let subworks = fpga_writer.lock().unwrap().subworks();
        if subworks.is_empty() {
            debug!("received: {}, but there is no subwork!", received.to_hex());
            return Ok(());
        }

        for sw2 in subworks {
            for i in (offset..16).chain(0..offset) {
                let version_bits = fpga::version_bits(sw2.vermask, version_count - i);
                let target = sw2.target(&nonce, version_bits);
                if target.starts_with(b"\0\0\0\0") {
                    offset = i;
                    let diff = Subwork2::target_diff(&target);
                    debug!("received: {}, difficulty: {:0<18}", received.to_hex(), diff);
                    if diff >= *pool_diff.lock().unwrap() {
                        let params = sw2.into_params("h723n8m.001", &nonce, version_bits);
                        let msg = Action {
                            id: Some(4),
                            method: String::from("mining.submit"),
                            params,
                        };
                        let data = msg.to_string().unwrap();
                        tokio::spawn(pool_sender.clone().send(data).then(|_| Ok(())));
                        info!(
                            "=> submit nonce: 0x{} (difficulty: {:0<18})",
                            nonce.to_hex(),
                            diff
                        );
                    };
                    return Ok(());
                }
            }
        }

        let crc_check = fpga::crc5_false(&received[0..7], 5) == received[6] & 0x1f;
        debug!(
            "received: {}, lost, crc check: {}",
            received.to_hex(),
            crc_check
        );
        Ok(())
    });

    //let mut runtime = current_thread::Runtime::new().unwrap();
    let task = connect_pool
        .select2(send_to_fpga)
        .select2(receive_nonce)
        .select2(nonce_reader)
        .select2(checker)
        .map(drop)
        .map_err(drop);
    //let _ = runtime.block_on(task);
    tokio::run(task);
}

fn main() {
    util::setup_logger().unwrap();

    thread::spawn(|| {
        let mut i2c = i2c::open("/dev/i2c-0");
        let addr = 0x55;
        loop {
            i2c.send_heart_beat(addr).expect("send heart beat err!");
            sleep(Duration::from_secs(10));
        }
    });

    loop {
        main_loop();
    }
}
