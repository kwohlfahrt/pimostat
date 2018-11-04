extern crate capnp;
extern crate clap;
use clap::{Arg, App};

use std::net::TcpListener;

include!(concat!(env!("OUT_DIR"), "/temperature_capnp.rs"));

fn main() {
    let matches = App::new("Temperature Sensor")
        .arg(Arg::with_name("port")
             .required(true)
             .index(1))
        .get_matches();

    let port: u16 = matches.value_of("port").unwrap().parse().unwrap();
    let listener = TcpListener::bind(("localhost", port)).unwrap();
    let read_opts = capnp::message::ReaderOptions::new();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        let reader = capnp::serialize::read_message(&mut stream, read_opts).unwrap();
        let msg = reader.get_root::<temperature::Reader>().unwrap();
        let temp = msg.get_value();

        println!("Read {}", temp);
    }
}
