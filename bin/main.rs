#![feature(bind_by_move_pattern_guards)]
#![allow(clippy::unreadable_literal)]

#[macro_use]
extern crate log;

use std::fs::File;
use std::process::exit;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;

use boardconfig::*;
use serde_json::{json, to_string as to_json_string};
use stratum::{stratum::*, util::*, work::*};
use tokio::prelude::*;
use tokio::runtime::current_thread;

fn main_loop(boards: Arc<Mutex<Vec<u16>>>, i2c: Arc<Mutex<I2c>>) {
    let config = &mut String::new();
    File::open("/etc/stratum/config.toml")
        .expect("can't open config.toml!")
        .read_to_string(config)
        .expect("can't read config.toml!");
    let config: Config = toml::from_str(&config).expect("can't parse config.toml!");

    // start init boards
    boards.lock().unwrap().clear();
    for id in &config.board.enabled {
        let (voltage, param) = config.board.get_setting(*id);
        init_board(*id, voltage, param, i2c.clone(), boards.clone()).expect("init board err!");
    }

    let exts = vec!["version-rolling"];
    let ext_params = json!({
        "version-rolling.mask": config.client.version_rolling.mask,
        "version-rolling.min-bit-count": config.client.version_rolling.min_bit_count
    });

    let mut pool0 = Pool::new(&config.pool[0].addr);
    let connect_pool = pool0.connect();
    let checker = pool0.checker();

    // mining.configure
    pool0.configure(exts, ext_params);
    pool0.subscribe(config.client.user_agent);
    pool0.authorize(&config.pool[0].user, &config.pool[0].pass);

    let pool_sender = pool0.sender();
    let pool_diff = pool0.diff.clone();

    let submitted_nonce = pool0.submitted_nonce.clone();

    let mut subwork2_stream = Subwork2Stream::default();
    let pool0_data = PoolData {
        duration: Duration::from_secs(30),
        works: WorkStream(pool0.work_channel.1),
        xnonce: pool0.xnonce.clone(),
        vermask: pool0.vermask.clone(),
        notify: pool0.work_notify.clone(),
        maker: None,
    };
    subwork2_stream.pools.push(pool0_data);

    let fpga_writer = Arc::new(Mutex::new(fpga::writer()));

    let fpga_writer_clone = fpga_writer.clone();
    let send_to_fpga = subwork2_stream.for_each(|(sw2, notify, timeout)| {
        //debug!("{:?}", &sw2);
        fpga_writer_clone.lock().unwrap().writer_subwork2(sw2);

        // TODO
        let notify_clone = notify.clone();
        notify
            .inspect(move |_| drop(notify_clone.notified()))
            .timeout(timeout)
            .then(|_| Ok(()))
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

    let mut offset = 0u32;
    let mut nonce_id = 0;
    let user = pool0.authorized.0.expect("not authorized!");
    let receive_nonce = nonce_receiver.for_each(move |received| {
        let fpga_writer = fpga_writer.clone();
        let nonce = u32::from_le_bytes(unsafe { *(received[0..4].as_ptr() as *const [u8; 4]) });
        let version_count =
            u32::from_le_bytes(unsafe { *(received[8..12].as_ptr() as *const [u8; 4]) })
                - u32::from(received[7].wrapping_sub(received[5]) & 0x7f);

        let subworks = fpga_writer.lock().unwrap().subworks();
        if subworks.is_empty() {
            debug!("received: {}, but there is no subwork!", received.to_hex());
            return Ok(());
        }

        for sw2 in subworks {
            for i in (1..=16).map(|x| {
                (if x & 1 == 0 {
                    offset.wrapping_add(x >> 1)
                } else {
                    offset.wrapping_sub(x >> 1)
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
                        nonce_id = nonce_id.wrapping_add(1);
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
    if pool0.connected.load(Ordering::SeqCst) && !pool0.authorized.1.load(Ordering::SeqCst) {
        exit(-1);
    }
}

fn main() {
    setup_logger().unwrap();

    let boards = Arc::new(Mutex::new(Vec::new()));
    let i2c = Arc::new(Mutex::new(i2c::open("/dev/i2c-0")));

    let boards_clone = boards.clone();
    let i2c_clone = i2c.clone();
    thread::spawn(move || {
        let i2c_lock = || {
            let i2c_lock = i2c_clone.lock().unwrap();
            sleep(Duration::from_micros(100));
            i2c_lock
        };
        loop {
            for id in &*boards_clone.lock().unwrap() {
                i2c_lock()
                    .send_heart_beat(0x50 + id)
                    .expect("send heart beat err!");
                sleep(Duration::from_micros(100));
            }
            sleep(Duration::from_secs(10));
        }
    });

    loop {
        main_loop(boards.clone(), i2c.clone());
    }
}
