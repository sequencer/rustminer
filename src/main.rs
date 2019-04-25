#![feature(bind_by_move_pattern_guards)]
#![allow(clippy::unreadable_literal)]

#[macro_use]
extern crate log;

use std::process::exit;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;

use serde_json::{json, to_string as to_json_string};
use tokio::prelude::*;
use tokio::runtime::current_thread;

use self::stratum::*;
use self::util::*;
use self::work::*;

pub mod stratum;
pub mod util;
pub mod work;

fn main_loop(boards: &[u16]) {
    let mut pool = Pool::new("121.29.19.24:443");
    let connect_pool = pool.connect();
    let checker = pool.checker();

    let exts = vec!["version-rolling"];
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
    let submitted_nonce = pool.submitted_nonce.clone();
    let vermask = pool.vermask.clone();
    let work_notify = pool.work_notify.clone();

    let mut fpga_writer = fpga::writer();
    for id in boards {
        fpga_writer.enable_sender(*id as usize);
    }
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

    let exit1 = Notify::default();
    let exit1_receiver = exit1.clone();
    let exit2 = Notify::default();
    let exit2_receiver = exit2.clone();

    thread::spawn(move || {
        let mut runtime = current_thread::Runtime::new().unwrap();
        let _ = runtime.block_on(nonce_reader.select2(exit1_receiver).then(|_| {
            exit2.notify();
            Result::<_, ()>::Ok(())
        }));
    });

    let mut offset = 0;
    let mut nonce_id = 0;
    let user = pool.authorized.0.expect("not authorized!");
    let receive_nonce = nonce_receiver.for_each(move |received| {
        let fpga_writer = fpga_writer.clone();
        let nonce = u32::from_le_bytes(unsafe { *(received[0..4].as_ptr() as *const [u8; 4]) });
        let version_count =
            u32::from_le_bytes(unsafe { *(received[8..12].as_ptr() as *const [u8; 4]) })
                - u32::from((received[7] - received[5]) & 0x7f);

        let subworks = fpga_writer.lock().unwrap().subworks();
        if subworks.is_empty() {
            debug!("received: {}, but there is no subwork!", received.to_hex());
            return Ok(());
        }

        for sw2 in subworks {
            for i in (1..=16).map(|x| {
                (if x & 1 == 0 {
                    offset + (x >> 1)
                } else {
                    offset - (x >> 1)
                }) & 0xf
            }) {
                let version_bits = fpga::version_bits(sw2.vermask, version_count - i);
                let target = sw2.target(nonce, version_bits);
                if target.starts_with(b"\0\0\0\0") {
                    offset = i;
                    let diff = Subwork2::target_diff(&target);
                    debug!("received: {}, difficulty: {:0<18}", received.to_hex(), diff);
                    if diff >= *pool_diff.lock().unwrap() {
                        let params = sw2.into_params(&user, nonce, version_bits);
                        let msg = Action {
                            id: Some(nonce_id),
                            method: "mining.submit",
                            params,
                        };

                        let data = to_json_string(&msg).unwrap();
                        tokio::spawn(pool_sender.clone().send(data).then(|_| Ok(())));
                        info!(
                            "=> submit nonce: 0x{:08x} (difficulty: {:0<18})",
                            nonce, diff
                        );

                        let submitted_nonce =
                            &mut submitted_nonce.lock().unwrap()[(nonce_id & 0b111) as usize];
                        if let Some(nonce_old) = submitted_nonce {
                            warn!("submitted nonce 0x{:08x} lost!", nonce_old);
                        }
                        *submitted_nonce = Some(nonce);
                        nonce_id += 1;
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

    let mut runtime = current_thread::Runtime::new().unwrap();
    let task = connect_pool
        .select2(send_to_fpga)
        .select2(receive_nonce)
        .select2(exit2_receiver.clone())
        .select2(checker)
        .then(move |_| {
            exit1.notify();
            exit2_receiver
        });
    let _ = runtime.block_on(task);

    // exit if authorized failed
    if pool.connected.load(Ordering::SeqCst) && !pool.authorized.1.load(Ordering::SeqCst) {
        exit(-1);
    }
}

fn main() {
    util::setup_logger().unwrap();

    let boards = &[5, 6];

    thread::spawn(move || {
        let mut i2c = i2c::open("/dev/i2c-0");
        loop {
            for id in boards {
                i2c.send_heart_beat(0x50 + id)
                    .expect("send heart beat err!");
                sleep(Duration::from_micros(100));
            }
            sleep(Duration::from_secs(10));
        }
    });

    loop {
        main_loop(boards);
    }
}
