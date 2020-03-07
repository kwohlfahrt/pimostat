extern crate tempfile;

use std::fs::{read, write};
use std::net::{Ipv6Addr, SocketAddr};
use std::thread::{sleep, spawn};
use std::time::Duration;

use pimostat::{actor, controller, sensor};

const COLD: &str = concat!(
    "a3 01 4b 46 7f ff 0e 10 d8 : crc=d8 YES\n",
    "a3 01 4b 46 7f ff 0e 10 d8 t=15123\n",
);

const WARM: &str = concat!(
    "a3 01 4b 46 7f ff 0e 10 d8 : crc=d8 YES\n",
    "a3 01 4b 46 7f ff 0e 10 d8 t=19123\n",
);

const HOT: &str = concat!(
    "a3 01 4b 46 7f ff 0e 10 d8 : crc=d8 YES\n",
    "a3 01 4b 46 7f ff 0e 10 d8 t=25123\n",
);

#[test]
fn test_all() {
    let w1_therm = tempfile::NamedTempFile::new().unwrap();
    let w1_therm_path = w1_therm.path().to_owned();
    write(w1_therm.path(), COLD.as_bytes()).unwrap();

    let gpio = tempfile::NamedTempFile::new().unwrap();
    let gpio_path = gpio.path().to_owned();

    spawn(move || sensor::run(5000.into(), &w1_therm_path, 1));
    spawn(move || {
        controller::run(
            5001.into(),
            SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), 5000),
            20.0,
            2.0,
        )
    });
    spawn(move || {
        actor::run(
            SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), 5001),
            &gpio_path,
        )
    });

    sleep(Duration::from_millis(500));
    write(w1_therm.path(), HOT.as_bytes()).unwrap();
    sleep(Duration::from_secs(1));
    write(w1_therm.path(), COLD.as_bytes()).unwrap();
    sleep(Duration::from_secs(1));
    write(w1_therm.path(), HOT.as_bytes()).unwrap();
    sleep(Duration::from_secs(1));
    write(w1_therm.path(), WARM.as_bytes()).unwrap();
    sleep(Duration::from_secs(1));
    write(w1_therm.path(), COLD.as_bytes()).unwrap();
    sleep(Duration::from_secs(1));
    assert_eq!(read(&gpio.path()).unwrap(), "1101001".as_bytes());
}
