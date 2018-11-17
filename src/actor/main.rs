extern crate capnp;
extern crate capnp_rpc;

extern crate clap;
use clap::{Arg, App};

extern crate tokio;
use tokio::io::AsyncRead;
// Capn'p clients are not Sync
use tokio::runtime::current_thread;

extern crate futures;
use futures::Future;

extern crate pimostat;
use pimostat::{Error, actor_capnp};

use std::net::SocketAddr;

struct Actor ();
impl actor_capnp::actor::Server for Actor {
    fn toggle(&mut self, _: actor_capnp::actor::ToggleParams,
               _: actor_capnp::actor::ToggleResults)
               -> capnp::capability::Promise<(), capnp::Error> {
        println!("Toggling actor!");
        capnp::capability::Promise::ok(())
    }
}

fn main() {
    let matches = App::new("Temperature Sensor")
        .arg(Arg::with_name("port")
             .required(true)
             .index(1))
        .get_matches();

    let port: u16 = matches.value_of("port").unwrap()
        .parse().expect("Invalid port");
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), port);

    let stream = tokio::net::TcpStream::connect(&addr)
        .map_err(Error::IO)
        .and_then(|s| {
            if let Err(e) = s.set_nodelay(true) {
                eprintln!("Warning: could not set nodelay ({})", e)
            };
            Ok(s.split())
        });

    let client = actor_capnp::actor::ToClient::new(Actor())
        .from_server::<capnp_rpc::Server>();

    let rpc_system = stream.and_then(|(reader, writer)| {
        let network = capnp_rpc::twoparty::VatNetwork::new(
            reader, writer, capnp_rpc::rpc_twoparty_capnp::Side::Server, Default::default()
        );
        capnp_rpc::RpcSystem::new(Box::new(network), Some(client.client))
            .map_err(Error::CapnP)
    });

    println!("Starting RPC system");
    current_thread::block_on_all(rpc_system)
        .expect("Failed to run RPC server");
}
