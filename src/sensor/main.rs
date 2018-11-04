extern crate capnp;
extern crate capnp_rpc;

extern crate clap;
use clap::{Arg, App};

use std::net::TcpStream;

#[allow(dead_code)]
mod temperature_capnp {
    include!(concat!(env!("OUT_DIR"), "/temperature_capnp.rs"));
}

fn main() {
    let matches = App::new("Thermostat Controller")
        .arg(Arg::with_name("port")
             .required(true)
             .index(1))
        .arg(Arg::with_name("temperature")
             .required(true)
             .index(2))
        .get_matches();

    let port: u16 = matches.value_of("port").unwrap()
        .parse().unwrap();
    let mut stream = TcpStream::connect(("localhost", port))
        .unwrap();

    let temperature: f32 = matches.value_of("temperature").unwrap()
        .parse().unwrap();

    let mut builder = capnp::message::Builder::new_default();
    {
        let mut msg = builder.init_root::<temperature_capnp::sensor_state::Builder>();
        msg.set_value(temperature);
    }

    capnp::serialize::write_message(&mut stream, &builder).unwrap();
}
