extern crate capnp;
extern crate clap;
use clap::{Arg, App};

use std::net::TcpListener;
use std::io::Read;

include!(concat!(env!("OUT_DIR"), "/temperature_capnp.rs"));

struct Config {
    pub target: f32,
    pub hysteresis: f32,
}

fn read_temperature<R: Read>(stream: &mut R) -> capnp::Result<f32> {
    let read_opts = capnp::message::ReaderOptions::new();
    let reader = capnp::serialize::read_message(stream, read_opts).unwrap();
    let msg = reader.get_root::<temperature::Reader>().unwrap();

    Ok(msg.get_value())
}

fn main() -> std::io::Result<()> {
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
    let listener = TcpListener::bind(("0.0.0.0", port))?;

    let cfg = Config {
        target: matches.value_of("target").unwrap()
            .parse().expect("Invalid temperature"),
        hysteresis: matches.value_of("hysteresis").unwrap_or("1.5")
            .parse().expect("Invalid hysteresis"),
    };

    let mut on: bool = false;

    for stream in listener.incoming() {
        let temp = read_temperature(&mut stream?).unwrap();

        if temp > cfg.target {
            on = false;
        } else if temp < (cfg.target - cfg.hysteresis) {
            on = true;
        }

        println!("Thermostat is {}", if on {"On"} else {"Off"});
    }

    Ok(())
}
