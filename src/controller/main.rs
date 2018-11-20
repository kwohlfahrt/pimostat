extern crate capnp;
extern crate capnp_futures;
extern crate capnp_rpc;

extern crate clap;
use clap::{Arg, App};

extern crate futures;
use futures::{Future, Stream};

extern crate tokio;
use tokio::io::AsyncRead;
// Capn'p clients are not Sync
use tokio::runtime::current_thread;

extern crate pimostat;
use pimostat::{Error, actor_capnp, controller_capnp};

use std::net::SocketAddr;

struct Config {
    pub target: f32,
    pub hysteresis: f32,
}

#[allow(unused)]
fn update(on: &mut bool, temperature: f32, cfg: &Config) {
    if temperature > cfg.target {
        *on = false;
    } else if temperature < (cfg.target - cfg.hysteresis) {
        *on = true;
    }
}

fn main() {
    let matches = App::new("Temperature Sensor")
        .arg(Arg::with_name("port")
             .required(true)
             .index(1))
        .arg(Arg::with_name("target")
             .required(true))
        .arg(Arg::with_name("hysteresis"))
        .get_matches();

    let port: u16 = matches.value_of("port").unwrap()
        .parse().expect("Invalid port");
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), port);

    let _cfg = Config {
        target: matches.value_of("target").unwrap()
            .parse().expect("Invalid temperature"),
        hysteresis: matches.value_of("hysteresis").unwrap_or("1.5")
            .parse().expect("Invalid hysteresis"),
    };
    let read_opts = capnp::message::ReaderOptions::new();

    let mut _on: bool = false;

    let listener = tokio::net::TcpListener::bind(&addr)
        .expect("Failed to bind to socket");

    let server = listener.incoming()
        .map_err(Error::IO)
        .map(|s| {
            println!("Accepted connection");
            if let Err(e)  = s.set_nodelay(true) {
                eprintln!("Warning: could not set nodelay ({})", e)
            };
            s.split()
        })
        .and_then(|(reader, writer)| {
            capnp_futures::serialize::read_message(reader, read_opts)
                .map_err(Error::CapnP)
                .map(|(reader, msg)|{
                    let value = msg.unwrap().get_root::<controller_capnp::hello::Reader>()
                        .unwrap().get_type().unwrap();
                    (reader, writer, value)
                })
        })
        .and_then(|(reader, writer, hello)| {
            match hello {
                controller_capnp::hello::Type::Sensor => {
                    unimplemented!()
                },
                controller_capnp::hello::Type::Actor => {
                    let network = capnp_rpc::twoparty::VatNetwork::new(
                        reader, writer, capnp_rpc::rpc_twoparty_capnp::Side::Client, Default::default()
                    );
                    let mut rpc_system = capnp_rpc::RpcSystem::new(Box::new(network), None);
                    let actor: actor_capnp::actor::Client =
                        rpc_system.bootstrap(capnp_rpc::rpc_twoparty_capnp::Side::Server);
                    current_thread::spawn(rpc_system.map_err(|e| eprintln!("RPC error ({})", e)));

                    actor.toggle_request().send().promise
                        .map_err(Error::CapnP)
                        .map(|_| println!("Received RPC Response"))
                },
            }
        }).for_each(|_| Ok(()));

    current_thread::block_on_all(server)
        .expect("Failed to run RPC client");
}
