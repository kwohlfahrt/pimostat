use std::path::Path;

use clap::{App, Arg};

use pimostat::controller::run;
use pimostat::error::Error;
use pimostat::util::split_host_port;

fn main() -> Result<(), Error> {
    let matches = App::new("Temperature Controller")
        .arg(Arg::with_name("no-tls").long("no-tls"))
        .arg(
            Arg::with_name("hysteresis")
                .long("hysteresis")
                .short("h")
                .takes_value(true),
        )
        .arg(Arg::with_name("certificate").long("cert").takes_value(true))
        .arg(Arg::with_name("sensor").required(true))
        .arg(Arg::with_name("temperature").required(true))
        .arg(Arg::with_name("address"))
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
    let address = matches.value_of("address").map(split_host_port);
    let sensor = matches.value_of("sensor").map(split_host_port).unwrap();
    let cert = matches.value_of("certificate").map(Path::new);

    run(
        address,
        cert,
        sensor,
        !matches.is_present("no-tls"),
        target,
        hysteresis,
    )
}
