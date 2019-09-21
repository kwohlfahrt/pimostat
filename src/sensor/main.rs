extern crate capnp;
extern crate capnp_rpc;

extern crate futures;
use futures::Future;

extern crate tokio;
use tokio::io::AsyncRead;
// Capn'p clients are not Sync
use tokio::runtime::current_thread;

extern crate clap;
use clap::{App, Arg};

extern crate pimostat;
use pimostat::{controller_capnp, sensor_capnp, Error};

use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::net::SocketAddr;

mod parse;
use parse::parse;

fn main() {
    let matches = App::new("Thermostat Sensor")
        .arg(Arg::with_name("port").required(true).index(1))
        .arg(Arg::with_name("source").required(true).index(2))
        .get_matches();

    let port: u16 = matches.value_of("port").unwrap().parse().unwrap();
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), port);

    let mut source = BufReader::new(File::open(matches.value_of("source").unwrap()).unwrap());

    let stream = tokio::net::TcpStream::connect(&addr)
        .map_err(Error::IO)
        .map(|s| s.split());

    let mut hello_builder = capnp::message::Builder::new_default();
    {
        let mut msg = hello_builder.init_root::<controller_capnp::hello::Builder>();
        msg.set_type(controller_capnp::hello::Type::Sensor);
    }

    let stream = stream.and_then(|(reader, writer)| {
        capnp_futures::serialize::write_message(writer, hello_builder)
            .map_err(Error::CapnP)
            .map(|(writer, _)| (reader, writer))
    });

    let mut msg_builder = capnp::message::Builder::new_default();
    {
        let mut msg = msg_builder.init_root::<sensor_capnp::sensor_state::Builder>();
        source.seek(SeekFrom::Start(0)).unwrap();
        msg.set_value(parse(&mut source).unwrap());
    }

    let stream = stream.and_then(|(reader, writer)| {
        capnp_futures::serialize::write_message(writer, msg_builder)
            .map_err(Error::CapnP)
            .map(|(writer, _)| (reader, writer))
    });

    current_thread::block_on_all(stream).expect("Failed to run RPC server");
}
