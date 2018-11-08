extern crate capnp;
extern crate capnp_rpc;
use capnp_rpc::pry;

extern crate futures;
use futures::future::Future;

extern crate clap;
use clap::{Arg, App};

use std::net::SocketAddr;
use std::net::TcpListener;

#[allow(dead_code)]
mod temperature_capnp {
    include!(concat!(env!("OUT_DIR"), "/temperature_capnp.rs"));
}

struct Sensor (pub f32);
impl temperature_capnp::sensor::Server for Sensor {
    fn measure(&mut self, _: temperature_capnp::sensor::MeasureParams,
               mut results: temperature_capnp::sensor::MeasureResults)
               -> capnp::capability::Promise<(), capnp::Error> {
        pry!(results.get().get_state()).set_value(self.0);
        capnp::capability::Promise::ok(())
    }
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
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), port);
    let listener = TcpListener::bind(&addr)
        .expect("Failed to bind to socket");

    let temperature: f32 = matches.value_of("temperature").unwrap()
        .parse().unwrap();

    let client = temperature_capnp::sensor::ToClient::new(Sensor(temperature))
        .from_server::<capnp_rpc::Server>();

    client.measure_request().send().promise.and_then(|r| {
        let temperature = r.get().unwrap().get_state()
            .unwrap().get_value();
        println!("Current temperature is {}", temperature);
        Ok(())
    }).wait().unwrap();

    let mut builder = capnp::message::Builder::new_default();
    {
        let mut msg = builder.init_root::<temperature_capnp::sensor_state::Builder>();
        msg.set_value(temperature);
    }

    // Listen
    for stream in listener.incoming() {
        let mut stream = stream.expect("Failed to accept connection");
        capnp::serialize::write_message(&mut stream, &builder).unwrap();
    }

}
