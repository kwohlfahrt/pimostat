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
use pimostat::{get_systemd_socket, actor_capnp, controller_capnp, sensor_capnp, Error};

use std::net::{SocketAddr, ToSocketAddrs};

#[derive(Copy, Clone)]
struct Config {
    pub target: f32,
    pub hysteresis: f32,
    pub port: Option<u16>,
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

struct Actor {
    actor: actor_capnp::actor::Client,
    pending: Option<
        Box<
            dyn Future<
                Item = capnp::capability::Response<actor_capnp::actor::toggle_results::Owned>,
                Error = capnp::Error,
            >,
        >,
    >,
}

// There would be fewer Box<dyn ...> with existential type aliases
struct State {
    config: Config,
    on: bool,
    actor: Option<Actor>,
    sensor: Box<dyn Stream<Item = f32, Error = Error>>,
    incoming: Box<dyn Stream<Item = actor_capnp::actor::Client, Error = Error>>,
}

impl State {
    fn new(config: Config) -> Result<Self, Error> {
	let listener = match config.port {
	    None => Ok(get_systemd_socket()),
	    Some(p) => {
		let addrs = [
		    SocketAddr::new("0.0.0.0".parse().unwrap(), p),
		    SocketAddr::new("::0".parse().unwrap(), p),
		];
		std::net::TcpListener::bind(&addrs[..])
	    },
	}?;

        let incoming = tokio::net::TcpListener::from_std(
		listener, &tokio::reactor::Handle::default()
	    )?
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
                            rpc_system.map_err(|e| eprintln!("RPC error ({})", e))
                        );

                        Ok(client)
                    })
            });

        let sensor = tokio::net::TcpStream::connect(&config.sensor)
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

        Ok(Self {
            config,
            on: false,
            actor: None,
            sensor: Box::new(sensor),
            incoming: Box::new(incoming),
        })
    }
}

impl Future for State {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
	let updated;

        match self.sensor.poll()? {
            Async::NotReady => {
		updated = false;
	    }
            Async::Ready(None) => return Ok(Async::Ready(())),
            Async::Ready(Some(value)) => {
		updated = true;
		update(&mut self.on, value, &self.config);
	    },
        }

        match self.actor.as_mut() {
            None => match self.incoming.poll()? {
                Async::NotReady => Ok(Async::NotReady),
                Async::Ready(None) => Ok(Async::Ready(())),
                Async::Ready(Some(a)) => {
                    self.actor = Some(Actor {
                        actor: a,
                        pending: None,
                    });
                    self.poll()
                }
            },
            Some(actor) => match actor.pending.as_mut() {
                None => if updated {
                    let mut req = actor.actor.toggle_request();
                    req.get().set_state(self.on);
                    actor.pending = Some(Box::new(req.send().promise));
                    self.poll()
                } else {
		    // self.sensor.poll() returned NotReady
		    Ok(Async::NotReady)
		},
                Some(p) => match p.poll() {
		    Ok(Async::NotReady) => Ok(Async::NotReady),
		    Ok(Async::Ready(_)) => {
			actor.pending = None;
			self.poll()
		    },
		    Err(e) => {
			if let capnp::ErrorKind::Disconnected = e.kind {
			    self.actor = None;
			    self.poll()
			} else {
			    Err(Error::CapnP(e))
			}
		    }
		},
            },
        }
    }
}

fn main() -> Result<(), std::io::Error> {
    let matches = App::new("Temperature Controller")
        .arg(Arg::with_name("port")
	     .short("p")
	     .long("port")
	     .takes_value(true))
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
        port: matches
	    .value_of("port")
	    .map(|p| p.parse().expect("Invalid port")),
        sensor: matches
            .value_of("sensor")
            .unwrap()
            .to_socket_addrs()?.next()
            .expect("Invalid sensor address"),
    };

    let state = State::new(cfg).expect("Failed to create state");

    println!("Starting RPC system");
    current_thread::block_on_all(state).expect("Failed to run RPC client");
    Ok(())
}
