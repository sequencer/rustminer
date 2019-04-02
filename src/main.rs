#![feature(const_int_conversion)]
#![allow(clippy::unreadable_literal)]

use std::iter::FromIterator;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::Bytes;
use serde_json::json;
use tokio::prelude::*;
use tokio::runtime::current_thread;
use tokio::timer::{Delay, Interval};

pub mod stratum;
pub mod util;
pub mod work;

use self::stratum::*;
use self::util::{
    fpga,
    i2c::{self, BoardConfig},
    ToHex,
};
use self::work::*;

fn main_loop() {
    let mut pool = Pool::new("121.29.19.24:443");
    let connect_pool = pool.connect();

    let exts = vec![
        String::from("minimum-difficulty"),
        String::from("version-rolling"),
    ];
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
    let has_new_work = pool.has_new_work.clone();

    let mut fpga_writer = fpga::writer();
    fpga_writer.enable_sender(0);
    let fpga_writer = Arc::new(Mutex::new(fpga_writer));

    let send_to_fpga = ws.for_each(|w| {
        let fpga_writer = fpga_writer.clone();
        Subwork2Maker::new(
            w,
            &xnonce.lock().unwrap(),
            vermask.lock().unwrap().unwrap(),
            has_new_work.clone(),
        )
        .for_each(move |sw2| {
            dbg!(&sw2);
            fpga_writer.lock().unwrap().writer_subwork2(sw2);
            Delay::new(Instant::now() + Duration::from_secs(10)).then(|_| Ok(()))
        })
        .then(|_| Ok(()))
    });

    let mut i2c = i2c::open("/dev/i2c-0");
    let addr = 0x55;
    i2c.send_heart_beat(addr).unwrap();
    let send_heart_beat = Interval::new_interval(Duration::from_secs(10))
        .map_err(|_| ())
        .for_each(move |_| i2c.send_heart_beat(addr).map_err(|e| eprintln!("{}", e)));

    let send_to_board = send_heart_beat.join(send_to_fpga);

    let mut offset = 0;
    let receive_nonce = fpga::reader()
        .receive_nonce()
        .map_err(|_| ())
        .for_each(|received| {
            if pool_sender.is_closed() {
                return Err(());
            };

            let fpga_writer = fpga_writer.clone();
            print!("received: {}", received.to_hex());
            let nonce = Bytes::from_iter(received[0..4].iter().rev().cloned());
            let version_count =
                u32::from_le_bytes(unsafe { *(received[8..12].as_ptr() as *const [u8; 4]) })
                    - u32::from(received[7] - received[5]);

            for sw2 in fpga_writer.lock().unwrap().subworks() {
                for i in (offset..16).chain(0..offset) {
                    let version_bits = fpga::version_bits(sw2.vermask, version_count - i);
                    let target = sw2.target(&nonce, version_bits);
                    if target.starts_with(b"\0\0\0\0") {
                        offset = i;
                        let pool_diff = pool_diff.clone();
                        let diff = Subwork2::target_diff(&target);
                        println!(", difficulty: {}", diff);
                        let pool_diff = pool_diff.lock().unwrap();
                        let pool_sender = pool_sender.clone();
                        if diff >= *pool_diff {
                            let params = sw2.into_params("h723n8m.001", &nonce, version_bits);
                            let msg = Action {
                                id: Some(4),
                                method: String::from("mining.submit"),
                                params,
                            };
                            let data = msg.to_string().unwrap();
                            tokio::spawn(pool_sender.send(data).then(|_| Ok(())));
                            println!("submit nonce: 0x{} (difficulty: {})", nonce.to_hex(), diff);
                        };
                        return Ok(());
                    }
                }
            }
            let crc_check = fpga::crc5_false(&received[0..7], 5) == received[6] & 0b00011111;
            println!(", lost, crc check: {}", crc_check);
            Ok(())
        });

    let mut runtime = current_thread::Runtime::new().unwrap();
    let _ = runtime.block_on(connect_pool.join(send_to_board).join(receive_nonce));
}

fn main() {
    loop {
        main_loop();
    }
}
