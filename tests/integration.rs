extern crate tempfile;

use std::path::Path;
use std::fs::{read, write};
use std::iter::repeat_with;
use std::net::{Ipv6Addr, SocketAddr};
use std::thread::{sleep, spawn};
use std::time::Duration;

use tempfile::NamedTempFile;

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
    let w1_therm = NamedTempFile::new().unwrap();
    let w1_therm_path = w1_therm.path().to_owned();
    write(w1_therm.path(), COLD.as_bytes()).unwrap();

    let gpios = repeat_with(|| NamedTempFile::new().unwrap())
        .take(4)
        .collect::<Vec<_>>();
    let gpio_paths = gpios
        .iter()
        .map(|gpio| gpio.path().to_owned())
        .collect::<Vec<_>>();

    spawn(move || sensor::run(5000.into(), &w1_therm_path, 1, None));

    (0..2).for_each(|i| {
        let port = 5010 + i;
        spawn(move || {
            controller::run(
                port.into(),
                SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 5000),
                20.0,
                2.0,
                None,
            )
        });
    });

    sleep(Duration::from_millis(250));
    gpio_paths
        .into_iter()
        .enumerate()
        .for_each(|(i, gpio_path)| {
            spawn(move || {
                let controller_port = (5010 + i / 2) as u16;
                actor::run(
                    SocketAddr::new(Ipv6Addr::LOCALHOST.into(), controller_port),
                    &gpio_path,
                )
            });
        });

    sleep(Duration::from_millis(250));
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

    assert_eq!(gpios.len(), 4);
    gpios
        .iter()
        .for_each(|gpio| assert_eq!(read(&gpio.path()).unwrap(), "101001".as_bytes()));
}

#[test]
fn test_ssl() {
    let w1_therm = NamedTempFile::new().unwrap();
    let w1_therm_path = w1_therm.path().to_owned();
    write(w1_therm.path(), COLD.as_bytes()).unwrap();

    let sensor = spawn(move || sensor::run(6000.into(), &w1_therm_path, 1, Some(Path::new("./tests/ssl/sensor.p12"))));
    let controller = spawn(move || {
	controller::run(
	    6001.into(),
	    SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 6000),
	    20.0,
	    2.0,
	    Some("sensor.example.com"),
	)
    });
    sensor.join().unwrap().unwrap();
    controller.join().unwrap().unwrap();
}
