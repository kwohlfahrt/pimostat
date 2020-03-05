extern crate capnp;
extern crate capnp_futures;
extern crate capnp_rpc;

extern crate clap;
use clap::{App, Arg};

extern crate futures;
use futures::{stream::unfold, Stream, TryFutureExt, TryStreamExt};

extern crate tokio;
use tokio::io::split;
use tokio::runtime;

extern crate tokio_util;
use tokio_util::compat::{Tokio02AsyncReadCompatExt, Tokio02AsyncWriteCompatExt};

extern crate pimostat;
use pimostat::{actor_capnp, controller_capnp, get_systemd_socket, sensor_capnp, Error};

use core::task::{Context, Poll};
use core::{future::Future, pin::Pin};
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
        Pin<
            Box<
                dyn Future<
                    Output = Result<
                        capnp::capability::Response<actor_capnp::actor::toggle_results::Owned>,
                        capnp::Error,
                    >,
                >,
            >,
        >,
    >,
}

struct State {
    config: Config,
    on: bool,
    actor: Option<Actor>,
    sensor: Pin<Box<dyn Stream<Item = Result<f32, Error>>>>,
    incoming: Pin<Box<dyn Stream<Item = Result<actor_capnp::actor::Client, Error>>>>,
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
            }
        }?;

        let incoming = tokio::net::TcpListener::from_std(listener)?
            .map_err(Error::IO)
            .and_then(|s| async {
                if let Err(e) = s.set_nodelay(true) {
                    eprintln!("Warning: could not set nodelay ({})", e)
                };

                let read_opts = capnp::message::ReaderOptions::new();
                let (mut reader, writer) = split(s);

                let msg =
                    capnp_futures::serialize::read_message((&mut reader).compat(), read_opts).await;
                msg.and_then(move |msg| {
                    msg.unwrap()
                        .get_root::<controller_capnp::hello::Reader>()?
                        .get_type()?;

                    let network = capnp_rpc::twoparty::VatNetwork::new(
                        reader.compat(),
                        writer.compat_write(),
                        capnp_rpc::rpc_twoparty_capnp::Side::Client,
                        Default::default(),
                    );

                    let mut rpc_system = capnp_rpc::RpcSystem::new(Box::new(network), None);
                    let client = rpc_system.bootstrap(capnp_rpc::rpc_twoparty_capnp::Side::Server);

                    tokio::task::spawn_local(
                        rpc_system.map_err(|e| eprintln!("RPC error ({})", e)),
                    );

                    Ok(client)
                })
                .map_err(Error::CapnP)
            });

        let sensor = tokio::net::TcpStream::connect(config.sensor)
            .map_err(Error::IO)
            .map_ok(|s| {
                let (reader, _) = split(s);
                // TODO: use capnp_futures::ReadStream
                unfold(reader, |mut reader| async {
                    let read_opts = capnp::message::ReaderOptions::new();
                    let msg =
                        capnp_futures::serialize::read_message((&mut reader).compat(), read_opts)
                            .await;

                    match msg {
                        Ok(Some(r)) => {
                            let temperature = r
                                .get_root::<sensor_capnp::state::Reader>()
                                .unwrap()
                                .get_value();
                            Some((Ok(temperature), reader))
                        }
                        Ok(None) => None,
                        Err(e) => Some((Err(Error::CapnP(e)), reader)),
                    }
                })
            })
            .try_flatten_stream();

        Ok(Self {
            config,
            on: false,
            actor: None,
            sensor: Box::pin(sensor),
            incoming: Box::pin(incoming),
        })
    }
}

impl Future for State {
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let updated;

        match self.sensor.as_mut().poll_next(cx) {
            Poll::Pending => {
                updated = false;
            }
            Poll::Ready(None) => return Poll::Ready(Ok(())),
            Poll::Ready(Some(value)) => {
                let value = value?;
                updated = true;
                let config = self.config; // Avoid overlapping borrow of fields
                update(&mut self.on, value, &config);
            }
        }

        let on = self.on; // Avoid overlapping borrow of fields
        match self.actor.as_mut() {
            None => match self.incoming.as_mut().poll_next(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(None) => Poll::Ready(Ok(())),
                Poll::Ready(Some(a)) => {
                    let a = a?;
                    self.actor = Some(Actor {
                        actor: a,
                        pending: None,
                    });
                    self.poll(cx)
                }
            },
            Some(actor) => match actor.pending.as_mut() {
                None => {
                    if updated {
                        let mut req = actor.actor.toggle_request();
                        req.get().set_state(on);
                        actor.pending = Some(Box::pin(req.send().promise));
                        self.poll(cx)
                    } else {
                        // self.sensor.poll(cx) returned Pending
                        Poll::Pending
                    }
                }
                Some(p) => match p.as_mut().poll(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Ok(_)) => {
                        actor.pending = None;
                        self.poll(cx)
                    }
                    Poll::Ready(Err(e)) => {
                        if let capnp::ErrorKind::Disconnected = e.kind {
                            self.actor = None;
                            self.poll(cx)
                        } else {
                            Poll::Ready(Err(Error::CapnP(e)))
                        }
                    }
                },
            },
        }
    }
}

fn main() -> Result<(), std::io::Error> {
    let matches = App::new("Temperature Controller")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true),
        )
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
            .to_socket_addrs()?
            .next()
            .expect("Invalid sensor address"),
    };

    let mut rt = runtime::Builder::new()
        .basic_scheduler()
        .enable_all()
        .build()
        .expect("Could not construct runtime");
    let local = tokio::task::LocalSet::new();

    let state = rt.enter(|| State::new(cfg).expect("Failed to create state"));
    println!("Starting RPC system");
    local
        .block_on(&mut rt, state)
        .expect("Failed to run RPC client");
    Ok(())
}
