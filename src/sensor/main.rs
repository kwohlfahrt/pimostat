extern crate capnp;
extern crate capnp_rpc;

extern crate futures;
use futures::Future;

extern crate tokio;
use tokio::io::AsyncRead;
// Capn'p clients are not Sync
use tokio::runtime::current_thread;

extern crate clap;
use clap::{Arg, App};

extern crate pimostat;
use pimostat::{Error, sensor_capnp, controller_capnp};

use std::net::SocketAddr;


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

    let temperature: f32 = matches.value_of("temperature").unwrap()
        .parse().unwrap();

    let stream = tokio::net::TcpStream::connect(&addr)
        .map_err(Error::IO)
        .map(|s| s.split());

    let mut hello_builder = capnp::message::Builder::new_default();
    {
        let mut msg = builder.init_root::<controller_capnp::hello::Builder>();
        msg.set_type(controller_capnp::hello::Type::Sensor);
    }

    let stream = stream
        .and_then(|(reader, writer)| {
            capnp_futures::serialize::write_message(writer, hello_builder)
                .map_err(Error::CapnP)
                .map(|(writer, _)| (reader, writer))
        });

    let mut msg_builder = capnp::message::Builder::new_default();
    {
        let mut msg = builder.init_root::<sensor_capnp::sensor_state::Builder>();
        msg.set_value(temperature);
    }

    let stream = stream
        .and_then(|(reader, writer)| {
            capnp_futures::serialize::write_message(writer, msg_builder)
                .map_err(Error::CapnP)
                .map(|(writer, _)| (reader, writer))
        });

    current_thread::block_on_all(server)
        .expect("Failed to run RPC server");
}
