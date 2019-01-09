use std::io::BufReader;
use std::path::Path;

use tokio::io;
use tokio_serial::{Serial, FlowControl, SerialPortSettings};

pub fn lines<P: AsRef<Path>>(path: P) -> io::Lines<BufReader<Serial>> {
    let mut s = SerialPortSettings::default();
    s.baud_rate = 115200;
    s.flow_control = FlowControl::Software;

    let mut port = Serial::from_path(path, &s).unwrap();
    #[cfg(unix)]
        port.set_exclusive(false).expect("set_exclusive(false) failed!");

    io::lines(BufReader::new(port))
}

#[test]
fn serial_println() {
    use tokio::prelude::*;

    #[cfg(unix)]
    const PORT: &str = "/dev/ttyUSB0";
    #[cfg(windows)]
    const PORT: &str = "COM1";

    let printer = lines(PORT)
        .for_each(|s| {
            println!("{:}", s);
            Ok(())
        }).map_err(|e| eprintln!("{}", e));

    tokio::run(printer);
}
