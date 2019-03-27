use std::net::Shutdown;
use std::time::Duration;

use tokio::timer::{Delay, Error, Interval};

use super::*;

pub fn new(pool: &mut Pool) -> impl Future<Item = (), Error = ()> + Send {
    let last_active = pool.last_active.clone();
    let tcpstream = pool.tcpstream.as_ref().unwrap().try_clone().unwrap();
    let has_new_work = pool.has_new_work.clone();
    let timeout = Duration::from_secs(60);
    Interval::new_interval(Duration::from_secs(1))
        .for_each(move |x| match *last_active.lock().unwrap() {
            Ok(t) if t + timeout >= x => Ok(()),
            _ => {
                eprintln!("pool connect timeout!");
                let _ = tcpstream.shutdown(Shutdown::Both);
                *has_new_work.lock().unwrap() = Some(());
                Err(Error::shutdown())
            }
        })
        .then(|_| Delay::new(Instant::now() + Duration::from_secs(15)))
        .map_err(|_| ())
}
