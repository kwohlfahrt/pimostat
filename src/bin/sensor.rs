use std::path::Path;

use clap::{App, Arg};

use pimostat::error::Error;
use pimostat::sensor::run;
use pimostat::util::split_host_port;

fn main() -> Result<(), Error> {
    let matches = App::new("Thermostat Sensor")
        .arg(
            Arg::with_name("interval")
                .long("interval")
                .short('i')
                .takes_value(true),
        )
        .arg(Arg::with_name("certificate").long("cert").takes_value(true))
        .arg(Arg::with_name("source").required(true))
        .arg(Arg::with_name("address"))
        .get_matches();

    let address = matches.value_of("address").map(split_host_port);
    let cert = matches.value_of("certificate").map(Path::new);
    let interval: u32 = matches
        .value_of("interval")
        .unwrap_or("60")
        .parse()
        .expect("Invalid interval");
    let source = matches.value_of("source").unwrap();

    run(address, cert, source.as_ref(), interval, None)
}
