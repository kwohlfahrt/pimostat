extern crate capnp;
extern crate capnp_futures;
extern crate capnp_rpc;

extern crate clap;
use clap::{App, Arg};

extern crate futures;
use futures::{stream, Async, Future, Poll, Stream};

extern crate tokio;
use tokio::io::AsyncRead;
// Capn'p clients are not Sync
use tokio::runtime::current_thread;

extern crate pimostat;
use pimostat::{actor_capnp, controller_capnp, sensor_capnp, Error};

use std::net::SocketAddr;

#[derive(Copy, Clone)]
struct Config {
    pub target: f32,
    pub hysteresis: f32,
    pub addr: SocketAddr,
    pub sensor: SocketAddr,
}

#[allow(unused)]
fn update(on: &mut bool, temperature: f32, cfg: &Config) {
    if temperature > cfg.target {
        *on = false;
    } else if temperature < (cfg.target - cfg.hysteresis) {
        *on = true;
    }
}

// There would be fewer Box<dyn ...> with existential type aliases
struct State {
    config: Config,
    on: bool,
    actor: Option<actor_capnp::actor::Client>,
    sensor: Option<Box<dyn Stream<Item = f32, Error = Error>>>,
    incoming: Box<dyn Stream<Item = actor_capnp::actor::Client, Error = Error>>,
}

impl State {
    fn new(config: Config) -> Result<Self, Error> {
        let incoming = tokio::net::TcpListener::bind(&config.addr)?
            .incoming()
            .map_err(Error::IO)
            .and_then(|s| {
                if let Err(e) = s.set_nodelay(true) {
                    eprintln!("Warning: could not set nodelay ({})", e)
                };

                let read_opts = capnp::message::ReaderOptions::new();
                capnp_futures::serialize::read_message(s, read_opts)
                    .map_err(Error::CapnP)
                    .and_then(|(s, msg)| {
                        msg.unwrap()
                            .get_root::<controller_capnp::hello::Reader>()?
                            .get_type()?;

                        let (reader, writer) = s.split();
                        let network = capnp_rpc::twoparty::VatNetwork::new(
                            reader,
                            writer,
                            capnp_rpc::rpc_twoparty_capnp::Side::Client,
                            Default::default(),
                        );

                        let mut rpc_system = capnp_rpc::RpcSystem::new(Box::new(network), None);
                        let client =
                            rpc_system.bootstrap(capnp_rpc::rpc_twoparty_capnp::Side::Server);

                        current_thread::spawn(
                            rpc_system.map_err(|e| eprintln!("RPC error ({})", e)),
                        );

                        Ok(client)
                    })
            });

        Ok(Self {
            config,
            on: false,
            actor: None,
            sensor: None,
            incoming: Box::new(incoming),
        })
    }
}

impl Future for State {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match (self.actor.as_ref(), self.sensor.as_mut()) {
            (None, _) => match self.incoming.poll()? {
                Async::Ready(None) => Ok(Async::Ready(())),
                Async::Ready(Some(s)) => {
                    self.actor = Some(s);
                    self.poll()
                }
                Async::NotReady => Ok(Async::NotReady),
            },
            (Some(_), None) => {
                let stream = tokio::net::TcpStream::connect(&self.config.sensor)
                    .map_err(Error::IO)
                    .map(move |s| {
                        stream::unfold(s, |s| {
                            let read_opts = capnp::message::ReaderOptions::new();
                            Some(
                                capnp_futures::serialize::read_message(s, read_opts)
                                    .map_err(Error::CapnP)
                                    .map(|(s, msg)| {
                                        (
                                            msg.unwrap()
                                                .get_root::<sensor_capnp::state::Reader>()
                                                .unwrap()
                                                .get_value(),
                                            s,
                                        )
                                    }),
                            )
                        })
                    })
                    .flatten_stream();
                self.sensor = Some(Box::new(stream));
                self.poll()
            }
            (Some(actor), Some(sensor)) => match sensor.poll()? {
                Async::Ready(None) => {
                    self.sensor = None;
                    self.poll()
                }
                Async::Ready(Some(value)) => {
                    update(&mut self.on, value, &self.config);
                    let mut req = actor.toggle_request();
                    req.get().set_state(self.on);
                    // FIXME: This is messy, need to wait for this to complete before sending next
                    current_thread::spawn(
                        req.send()
                            .promise
                            .map_err(|e| eprintln!("RPC error: {}", e))
                            .map(|_| ()),
                    );
                    self.poll()
                }
                Async::NotReady => Ok(Async::NotReady),
            },
        }
    }
}

fn main() {
    let matches = App::new("Temperature Controller")
        .arg(Arg::with_name("port").required(true).index(1))
        .arg(Arg::with_name("sensor").required(true))
        .arg(Arg::with_name("temperature").required(true))
        .arg(Arg::with_name("hysteresis"))
        .get_matches();

    let cfg = Config {
        target: matches
            .value_of("temperature")
            .unwrap()
            .parse()
            .expect("Invalid temperature"),
        hysteresis: matches
            .value_of("hysteresis")
            .unwrap_or("1.5")
            .parse()
            .expect("Invalid hysteresis"),
        addr: SocketAddr::new(
            "0.0.0.0".parse().unwrap(),
            matches
                .value_of("port")
                .unwrap()
                .parse()
                .expect("Invalid port"),
        ),
        sensor: matches
            .value_of("sensor")
            .unwrap()
            .parse::<SocketAddr>()
            .expect("Invalid sensor address"),
    };

    let state = State::new(cfg).expect("Failed to create state");

    println!("Starting RPC system");
    current_thread::block_on_all(state).expect("Failed to run RPC client");
}
