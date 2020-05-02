extern crate clap;

use std::convert::TryFrom;
use std::path::Path;

use clap::{App, Arg};

use pimostat::actor::{run, GpioActor};
use pimostat::error::Error;
use pimostat::util::split_host_port;

fn main() -> Result<(), Error> {
    let matches = App::new("Temperature Actor")
        .arg(Arg::with_name("no-tls").long("no-tls"))
        .arg(Arg::with_name("controller").required(true))
        .arg(Arg::with_name("chip").required(true))
        .arg(Arg::with_name("line").required(true))
        .get_matches();

    let controller = matches.value_of("controller").map(split_host_port).unwrap();
    let chip: &Path = matches.value_of("chip").unwrap().as_ref();
    let line: u32 = matches.value_of("line").unwrap().parse().unwrap();

    run(
        controller,
        !matches.is_present("no-tls"),
        GpioActor::try_from((chip, line))?,
    )
}
