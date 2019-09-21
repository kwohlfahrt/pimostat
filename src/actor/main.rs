extern crate capnp;
extern crate capnp_futures;
extern crate capnp_rpc;

use capnp_rpc::pry;

extern crate clap;
use clap::{Arg, App};

extern crate tokio;
use tokio::io::AsyncRead;
// Capn'p clients are not Sync
use tokio::runtime::current_thread;

extern crate futures;
use futures::Future;

extern crate pimostat;
use pimostat::{Error, actor_capnp, controller_capnp};

use std::net::SocketAddr;
use std::fs::{File, OpenOptions};
use std::io::Write;

struct Actor {
    gpio: File
}

impl actor_capnp::actor::Server for Actor {
    fn toggle(&mut self, params: actor_capnp::actor::ToggleParams,
               _: actor_capnp::actor::ToggleResults)
               -> capnp::capability::Promise<(), capnp::Error> {
        let state = pry!(params.get()).get_state();
        match write!(self.gpio, "{}", if state {"1"} else {"0"}) {
            Ok(()) => capnp::capability::Promise::ok(()),
            Err(e) => capnp::capability::Promise::err(capnp::Error::failed(format!("{}", e))),
        }
    }
}

fn main() {
    let matches = App::new("Temperature Actor")
        .arg(Arg::with_name("port")
             .required(true)
             .index(1))
        .arg(Arg::with_name("GPIO")
             .required(true)
             .index(2))
        .get_matches();

    let port: u16 = matches.value_of("port").unwrap()
        .parse().expect("Invalid port");
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), port);
    let gpio = OpenOptions::new().read(false).write(true)
        .open(matches.value_of("GPIO").unwrap()).unwrap();

    let stream = tokio::net::TcpStream::connect(&addr)
        .map_err(Error::IO)
        .map(|s| {
            if let Err(e) = s.set_nodelay(true) {
                eprintln!("Warning: could not set nodelay ({})", e)
            };
            s.split()
        });

    let client = actor_capnp::actor::ToClient::new(Actor {gpio})
        .into_client::<capnp_rpc::Server>();

    let mut builder = capnp::message::Builder::new_default();
    {
        let mut msg = builder.init_root::<controller_capnp::hello::Builder>();
        msg.set_type(controller_capnp::hello::Type::Actor);
    }

    let rpc_system = stream
        .and_then(|(reader, writer)| {
            capnp_futures::serialize::write_message(writer, builder)
                .map_err(Error::CapnP)
                .map(|(writer, _)| (reader, writer))
        })
        .and_then(|(reader, writer)| {
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
