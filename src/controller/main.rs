extern crate capnp;
extern crate capnp_rpc;

extern crate clap;
use clap::{Arg, App};

use std::net::SocketAddr;
use std::net::TcpStream;

#[allow(dead_code)]
mod temperature_capnp {
    include!(concat!(env!("OUT_DIR"), "/temperature_capnp.rs"));
}

struct Config {
    pub target: f32,
    pub hysteresis: f32,
}

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

    let cfg = Config {
        target: matches.value_of("target").unwrap()
            .parse().expect("Invalid temperature"),
        hysteresis: matches.value_of("hysteresis").unwrap_or("1.5")
            .parse().expect("Invalid hysteresis"),
    };
    let read_opts = capnp::message::ReaderOptions::new();

    let mut on: bool = false;

    let mut stream = TcpStream::connect(&addr)
        .expect("Failed to connect to socket");
    let msg = capnp::serialize::read_message(&mut stream, read_opts)
        .expect("Failed to read message");
    let contents = msg.get_root::<temperature_capnp::sensor_state::Reader>()
        .expect("Failed to get message contents");
    let temperature = contents.get_value();
    update(&mut on, temperature, &cfg);
    println!("Thermostat is {}", if on {"On"} else {"Off"});
}
