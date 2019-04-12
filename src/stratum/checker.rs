use std::net::Shutdown;
use std::time::Duration;

use tokio::timer::{Error, Interval};

use super::*;

impl Pool {
    pub fn checker(&mut self) -> impl Future<Item = (), Error = ()> + Send {
        let last_active = self.last_active.clone();
        let tcpstream = self.tcpstream.as_ref().unwrap().try_clone().unwrap();
        let timeout = Duration::from_secs(60);
        Interval::new_interval(Duration::from_secs(1))
            .for_each(move |x| match *last_active.lock().unwrap() {
                Ok(t) if t + timeout >= x => Ok(()),
                _ => {
                    error!("pool connect timeout!");
                    let _ = tcpstream.shutdown(Shutdown::Both);
                    Err(Error::shutdown())
                }
            })
            .map_err(|_| ())
    }
}
