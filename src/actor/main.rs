extern crate clap;
use clap::{App, Arg};
use pimostat::actor::run;
use pimostat::error::Error;

use std::net::ToSocketAddrs;

fn main() -> Result<(), Error> {
    let matches = App::new("Temperature Actor")
        .arg(Arg::with_name("controller").required(true).index(1))
        .arg(Arg::with_name("GPIO").required(true).index(2))
        .get_matches();

    let addr = matches
        .value_of("controller")
        .unwrap()
        .to_socket_addrs()?
        .next()
        .expect("Invalid controller address");
    let gpio = matches.value_of("GPIO").unwrap();

    run(addr, gpio)
}
