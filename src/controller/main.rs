extern crate capnp;
extern crate capnp_rpc;

extern crate clap;
use clap::{Arg, App};

extern crate futures;
use futures::Future;

extern crate tokio;
use tokio::io::AsyncRead;
// Capn'p clients are not Sync
use tokio::runtime::current_thread;

use std::net::SocketAddr;

#[allow(dead_code)]
mod temperature_capnp {
    include!(concat!(env!("OUT_DIR"), "/temperature_capnp.rs"));
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

    let mut runtime = current_thread::Runtime::new()
        .expect("Failed to start runtime");

    let stream = runtime.block_on(
        tokio::net::TcpStream::connect(&addr)
    ).expect("Failed to connect to socket");

    if let Err(e)  = stream.set_nodelay(true) {
        eprintln!("Warning: could not set nodelay ({})", e)
    };
    let (reader, writer) = stream.split();
    let network = capnp_rpc::twoparty::VatNetwork::new(
        reader, writer, capnp_rpc::rpc_twoparty_capnp::Side::Client, Default::default()
    );
    let mut rpc_system = capnp_rpc::RpcSystem::new(Box::new(network), None);

    // TODO: read about this
    let sensor: temperature_capnp::sensor::Client =
        rpc_system.bootstrap(capnp_rpc::rpc_twoparty_capnp::Side::Server);

    runtime.spawn(rpc_system.map_err(|e| eprintln!("RPC error ({})", e)));

    let result = runtime.block_on(sensor.measure_request().send().promise)
        .expect("Error sending RPC request");
    let temperature = result.get()
        .expect("Error reading RPC result").get_state()
        .expect("Error reading sensor state").get_value();

    println!("Temperature is: {}", temperature);
}
