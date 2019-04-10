use std::net::Shutdown;
use std::time::Duration;

use tokio::timer::{Delay, Error, Interval};

use super::*;

impl Pool {
    pub(super) fn checker(&mut self) -> impl Future<Item = (), Error = ()> + Send {
        let last_active = self.last_active.clone();
        let tcpstream = self.tcpstream.as_ref().unwrap().try_clone().unwrap();
        let work_notify = self.work_notify.clone();
        let timeout = Duration::from_secs(60);
        Interval::new_interval(Duration::from_secs(1))
            .for_each(move |x| match *last_active.lock().unwrap() {
                Ok(t) if t + timeout >= x => Ok(()),
                _ => {
                    eprintln!("pool connect timeout!");
                    let _ = tcpstream.shutdown(Shutdown::Both);
                    Err(Error::shutdown())
                }
            })
            .then(|_| {
                Delay::new(Instant::now() + Duration::from_secs(15)).and_then(move |_| {
                    work_notify.notify();
                    Ok(())
                })
            })
            .map_err(|_| ())
    }
}
