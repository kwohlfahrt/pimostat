extern crate capnp;
extern crate capnp_rpc;
use capnp_rpc::pry;

extern crate futures;
use futures::{Stream, Future};

extern crate tokio;
use tokio::io::AsyncRead;
use tokio::net::TcpListener;
// Capn'p clients are not Sync
use tokio::runtime::current_thread;

extern crate clap;
use clap::{Arg, App};

extern crate pimostat;
use pimostat::{temperature_capnp};

use std::net::SocketAddr;

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
    let server = listener.incoming().for_each(|s| {
        if let Err(e)  = s.set_nodelay(true) {
            eprintln!("Warning: could not set nodelay ({})", e)
        };

        let (reader, writer) = s.split();

        let network = capnp_rpc::twoparty::VatNetwork::new(
            reader, writer, capnp_rpc::rpc_twoparty_capnp::Side::Server, Default::default()
        );
        let rpc_system = capnp_rpc::RpcSystem::new(
            Box::new(network), Some(client.clone().client)
        );
        current_thread::spawn(rpc_system.map_err(
            |e| {eprintln!("Error in RPC system ({})", e)}
        ));
        Ok(())
    });

    current_thread::block_on_all(server)
        .expect("Failed to run RPC server");
}
