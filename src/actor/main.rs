extern crate capnp;
extern crate clap;
use clap::{Arg, App};

use std::net::TcpListener;
use std::io::Read;
use std::str;

include!(concat!(env!("OUT_DIR"), "/temperature_capnp.rs"));

fn main() {
    let matches = App::new("Temperature Sensor")
        .arg(Arg::with_name("port")
             .required(true)
             .index(1))
        .get_matches();

    let port: u16 = matches.value_of("port").unwrap().parse().unwrap();
    let listener = TcpListener::bind(("localhost", port)).unwrap();

    for stream in listener.incoming() {
        let mut buffer = Vec::new();
        let mut stream = stream.unwrap();
        stream.read_to_end(&mut buffer).unwrap();

        let data = str::from_utf8(&buffer).unwrap();
        println!("Read {}", data);
    }
}
