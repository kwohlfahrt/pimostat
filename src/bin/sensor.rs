extern crate clap;

use std::path::Path;

use clap::{App, Arg};

use pimostat::error::Error;
use pimostat::sensor::run;

fn main() -> Result<(), Error> {
    let matches = App::new("Thermostat Sensor")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true),
        )
        .arg(Arg::with_name("source").required(true))
        .arg(Arg::with_name("interval").required(true))
        .arg(Arg::with_name("certificate").long("cert").takes_value(true))
        .get_matches();

    let port: Option<u16> = matches
        .value_of("port")
        .map(|p| p.parse().expect("Invalid port"));
    let cert = matches.value_of("certificate").map(Path::new);
    let interval: u32 = matches
        .value_of("interval")
        .unwrap()
        .parse()
        .expect("Invalid interval");
    let source = matches.value_of("source").unwrap();

    run(port, source.as_ref(), interval, cert)
}
