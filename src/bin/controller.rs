extern crate clap;

use std::net::ToSocketAddrs;

use clap::{App, Arg};

use pimostat::controller::run;
use pimostat::error::Error;

fn main() -> Result<(), Error> {
    let matches = App::new("Temperature Controller")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true),
        )
        .arg(Arg::with_name("no-tls").long("no-tls"))
        .arg(Arg::with_name("sensor").required(true))
        .arg(Arg::with_name("temperature").required(true))
        .arg(Arg::with_name("hysteresis"))
        .get_matches();

    let target: f32 = matches
        .value_of("temperature")
        .unwrap()
        .parse()
        .expect("Invalid temperature");
    let hysteresis: f32 = matches
        .value_of("hysteresis")
        .unwrap_or("1.5")
        .parse()
        .expect("Invalid hysteresis");
    let port: Option<u16> = matches
        .value_of("port")
        .map(|p| p.parse().expect("Invalid port"));
    let sensor = matches
        .value_of("sensor")
        .unwrap()
        .to_socket_addrs()?
        .next()
        .expect("Invalid sensor address");

    let tls_url = if matches.is_present("no-tls") {
        None
    } else {
        let sensor = matches.value_of("sensor").unwrap();
        let (url, _) = sensor.split_at(sensor.rfind(":").unwrap());
        Some(url)
    };

    run(port, sensor, target, hysteresis, tls_url)
}
