extern crate capnp;
extern crate clap;
use clap::{Arg, App};

use std::net::TcpStream;
use std::io::Write;

include!(concat!(env!("OUT_DIR"), "/temperature_capnp.rs"));

fn main() {
    let matches = App::new("Thermostat Controller")
        .arg(Arg::with_name("port")
             .required(true)
             .index(1))
        .get_matches();

    let port: u16 = matches.value_of("port").unwrap().parse().unwrap();
    let mut stream = TcpStream::connect(("localhost", port))
        .unwrap();

    let data = "foo";
    println!("Writing {}", data);
    stream.write_all(data.as_bytes()).unwrap();
}
