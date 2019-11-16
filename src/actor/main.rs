extern crate capnp;
extern crate capnp_futures;
extern crate capnp_rpc;

use capnp_rpc::pry;

extern crate clap;
use clap::{App, Arg};

extern crate tokio;
use tokio::io::AsyncRead;
// Capn'p clients are not Sync
use tokio::runtime::current_thread;

extern crate futures;
use futures::Future;

extern crate pimostat;
use pimostat::{actor_capnp, controller_capnp, Error};

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::net::ToSocketAddrs;

struct Actor {
    gpio: File,
}

impl Actor {
    fn update(&mut self, state: bool) -> std::io::Result<()> {
	write!(self.gpio, "{}", if state { "1" } else { "0" })?;
	self.gpio.flush()?;
	Ok(())
    }
}

impl actor_capnp::actor::Server for Actor {
    fn toggle(
        &mut self,
        params: actor_capnp::actor::ToggleParams,
        _: actor_capnp::actor::ToggleResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let state = pry!(params.get()).get_state();
        match self.update(state) {
            Ok(()) => capnp::capability::Promise::ok(()),
            Err(e) => capnp::capability::Promise::err(capnp::Error::failed(format!("{}", e))),
        }
    }
}

fn main() -> Result<(), std::io::Error> {
    let matches = App::new("Temperature Actor")
        .arg(Arg::with_name("controller").required(true).index(1))
        .arg(Arg::with_name("GPIO").required(true).index(2))
        .get_matches();

    let addr = matches
        .value_of("controller")
        .unwrap()
        .to_socket_addrs()?.next()
        .expect("Invalid controller address");
    let gpio = OpenOptions::new()
        .read(false)
        .write(true)
        .open(matches.value_of("GPIO").unwrap())
        .unwrap();

    let client =
        actor_capnp::actor::ToClient::new(Actor { gpio }).into_client::<capnp_rpc::Server>();

    let mut builder = capnp::message::Builder::new_default();
    {
        let mut msg = builder.init_root::<controller_capnp::hello::Builder>();
        msg.set_type(controller_capnp::hello::Type::Actor);
    }

    let rpc_system = tokio::net::TcpStream::connect(&addr)
        .map_err(Error::IO)
        .and_then(|s| {
            if let Err(e) = s.set_nodelay(true) {
                eprintln!("Warning: could not set nodelay ({})", e)
            };
            let (reader, writer) = s.split();
            capnp_futures::serialize::write_message(writer, builder)
                .map_err(Error::CapnP)
                .map(|(writer, _)| (reader, writer))
        })
        .and_then(|(reader, writer)| {
            let network = capnp_rpc::twoparty::VatNetwork::new(
                reader,
                writer,
                capnp_rpc::rpc_twoparty_capnp::Side::Server,
                Default::default(),
            );
            capnp_rpc::RpcSystem::new(Box::new(network), Some(client.client)).map_err(Error::CapnP)
        });

    println!("Starting RPC system");
    current_thread::block_on_all(rpc_system).expect("Failed to run RPC server");
    Ok(())
}
