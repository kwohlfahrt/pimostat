extern crate capnp;
extern crate capnp_rpc;

extern crate clap;
use clap::{Arg, App};

extern crate futures;
use futures::{Future, Stream};

extern crate tokio;
use tokio::io::AsyncRead;
// Capn'p clients are not Sync
use tokio::runtime::current_thread;

use std::net::SocketAddr;

#[allow(dead_code)]
mod actor_capnp {
    include!(concat!(env!("OUT_DIR"), "/actor_capnp.rs"));
}

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

#[derive(Debug)]
enum Error{
    CapnP(capnp::Error),
    IO(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Error::IO(e) => write!(fmt, "IO({})", e),
            Error::CapnP(e) => write!(fmt, "CapnP({})", e),
        }
    }
}

impl From<capnp::Error> for Error {
    fn from(e: capnp::Error) -> Self {
        Error::CapnP(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO(e)
    }
}

impl std::error::Error for Error {}

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
    let _read_opts = capnp::message::ReaderOptions::new();

    let mut _on: bool = false;

    let listener = tokio::net::TcpListener::bind(&addr)
        .expect("Failed to bind to socket");

    let server = listener.incoming()
        .map(|s| {
            println!("Accepted connection");
            if let Err(e)  = s.set_nodelay(true) {
                eprintln!("Warning: could not set nodelay ({})", e)
            };
            let (reader, writer) = s.split();

            let network = capnp_rpc::twoparty::VatNetwork::new(
                reader, writer, capnp_rpc::rpc_twoparty_capnp::Side::Client, Default::default()
            );
            let mut rpc_system = capnp_rpc::RpcSystem::new(Box::new(network), None);
            let actor: actor_capnp::actor::Client =
                rpc_system.bootstrap(capnp_rpc::rpc_twoparty_capnp::Side::Server);

            current_thread::spawn(rpc_system.map_err(|e| eprintln!("RPC error ({})", e)));
            println!("Spawned RPC system");
            actor.toggle_request().send().promise
        }).for_each(|r| {
            current_thread::spawn(
                r.map_err(|e| eprintln!("RPC error ({})", e))
                    .map(|_| println!("Received RPC Response"))
            );
            Ok(())
        }).map_err(Error::IO);

    current_thread::block_on_all(server)
        .expect("Failed to run RPC client");
}
