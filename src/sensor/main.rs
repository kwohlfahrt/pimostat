extern crate capnp;
extern crate clap;
use clap::{Arg, App};

use std::net::TcpStream;

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

    let mut builder = capnp::message::Builder::new_default();
    {
        let mut msg = builder.init_root::<temperature::Builder>();
        msg.set_value(21.5);
    }

    capnp::serialize::write_message(&mut stream, &builder).unwrap();
}
