extern crate tempfile;

use std::convert::TryFrom;
use std::env;
use std::fs::{read, write};
use std::iter::repeat_with;
use std::path::Path;
use std::thread::{sleep, spawn};
use std::time::Duration;

use futures::channel::oneshot;
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

    let (tx, rx) = oneshot::channel();

    let sensor = spawn(move || sensor::run(Some(("::1", 5000)), None, &w1_therm_path, 1, Some(rx)));

    let controllers = (0..2)
        .map(|i| {
            let port = 5010 + i;
            spawn(move || {
                controller::run(Some(("::1", port)), None, ("::1", 5000), false, 20.0, 2.0)
            })
        })
        .collect::<Vec<_>>();

    sleep(Duration::from_millis(250));
    let actors = gpio_paths
        .into_iter()
        .enumerate()
        .map(|(i, gpio_path)| {
            spawn(move || {
                let controller_port = (5010 + i / 2) as u16;
                actor::run(
                    ("::1", controller_port),
                    false,
                    actor::FileActor::try_from(gpio_path.as_ref()).unwrap(),
                )
            })
        })
        .collect::<Vec<_>>();

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

    tx.send(()).unwrap();
    sensor.join().unwrap().unwrap();
    controllers
        .into_iter()
        .for_each(|handle| handle.join().unwrap().unwrap());
    actors
        .into_iter()
        .for_each(|handle| handle.join().unwrap().unwrap());

    assert_eq!(gpios.len(), 4);
    gpios
        .iter()
        .for_each(|gpio| assert_eq!(read(&gpio).unwrap(), "101001".as_bytes()));
}

#[test]
fn test_ssl() {
    let cert = Path::new("./tests/ssl/localhost.p12");

    let w1_therm = NamedTempFile::new().unwrap();
    let w1_therm_path = w1_therm.path().to_owned();
    write(w1_therm.path(), COLD.as_bytes()).unwrap();

    let gpio = NamedTempFile::new().unwrap();
    let gpio_path = gpio.path().to_owned();

    env::set_var("SSL_CERT_FILE", "./tests/ssl/root/cert.pem");

    let (tx, rx) = oneshot::channel();

    let sensor = spawn(move || {
        sensor::run(
            Some(("localhost", 6000)),
            Some(cert),
            &w1_therm_path,
            1,
            Some(rx),
        )
    });

    let controller = spawn(move || {
        controller::run(
            Some(("localhost", 6001)),
            Some(cert),
            ("localhost", 6000),
            true,
            20.0,
            2.0,
        )
    });

    sleep(Duration::from_millis(250));
    let actor = spawn(move || {
        actor::run(
            ("localhost", 6001),
            true,
            actor::FileActor::try_from(gpio_path.as_ref()).unwrap(),
        )
    });

    sleep(Duration::from_millis(250));
    write(w1_therm.path(), HOT.as_bytes()).unwrap();
    sleep(Duration::from_secs(1));

    tx.send(()).unwrap();
    sensor.join().unwrap().unwrap();
    controller.join().unwrap().unwrap();
    actor.join().unwrap().unwrap();
    assert_eq!(read(&gpio).unwrap(), "10".as_bytes());
}
