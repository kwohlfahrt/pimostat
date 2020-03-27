extern crate tempfile;

use std::env;
use std::fs::{read, write};
use std::iter::repeat_with;
use std::path::Path;
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

    spawn(move || sensor::run(Some(("::1", 5000)), None, &w1_therm_path, 1));

    (0..2).for_each(|i| {
        let port = 5010 + i;
        spawn(move || controller::run(Some(("::1", port)), None, ("::1", 5000), false, 20.0, 2.0));
    });

    sleep(Duration::from_millis(250));
    gpio_paths
        .into_iter()
        .enumerate()
        .for_each(|(i, gpio_path)| {
            spawn(move || {
                let controller_port = (5010 + i / 2) as u16;
                actor::run(("::1", controller_port), false, &gpio_path)
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

    let gpio = NamedTempFile::new().unwrap();
    let gpio_path = gpio.path().to_owned();

    env::set_var("SSL_CERT_FILE", "./tests/ssl/root/cert.pem");

    spawn(move || {
        sensor::run(
            Some(("::1", 6000)),
            Some(Path::new("./tests/ssl/localhost.p12")),
            &w1_therm_path,
            1,
        )
    });

    spawn(move || {
        controller::run(
            Some(("::1", 6001)),
            None,
            ("localhost", 6000),
            true,
            20.0,
            2.0,
        )
    });

    sleep(Duration::from_millis(250));
    spawn(move || {
        actor::run(
            ("::1", 6001),
            // TODO: Test TLS for actor
            false,
            &gpio_path,
        )
    });

    sleep(Duration::from_millis(250));
    write(w1_therm.path(), HOT.as_bytes()).unwrap();
    sleep(Duration::from_secs(1));
    assert_eq!(read(&gpio.path()).unwrap(), "10".as_bytes());
}
