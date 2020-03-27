extern crate clap;

use clap::{App, Arg};

use pimostat::actor::run;
use pimostat::error::Error;
use pimostat::util::split_host_port;

fn main() -> Result<(), Error> {
    let matches = App::new("Temperature Actor")
        .arg(Arg::with_name("no-tls").long("no-tls"))
        .arg(Arg::with_name("controller").required(true).index(1))
        .arg(Arg::with_name("GPIO").required(true).index(2))
        .get_matches();

    let controller = matches.value_of("controller").map(split_host_port).unwrap();
    let gpio = matches.value_of("GPIO").unwrap();

    run(controller, !matches.is_present("no-tls"), gpio.as_ref())
}
