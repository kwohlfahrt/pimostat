extern crate capnp;
extern crate capnp_futures;
extern crate capnp_rpc;
extern crate futures;
extern crate tokio;
extern crate tokio_util;

use std::net::SocketAddr;

use capnp_futures::serialize::read_message;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use tokio::io::split;
use tokio::runtime;
use tokio::sync::watch::channel;
use tokio_util::compat::{Tokio02AsyncReadCompatExt, Tokio02AsyncWriteCompatExt};

use crate::error::Error;
use crate::socket::listen_on;
use crate::{actor_capnp, controller_capnp, sensor_capnp};

#[derive(Copy, Clone)]
struct Config {
    pub target: f32,
    pub hysteresis: f32,
    pub port: Option<u16>,
    pub sensor: SocketAddr,
}

#[allow(unused)]
fn update(on: &mut bool, temperature: f32, target: f32, hysteresis: f32) {
    if temperature > target {
        *on = false;
    } else if temperature < (target - hysteresis) {
        *on = true;
    }
}

pub fn run(
    port: Option<u16>,
    sensor: SocketAddr,
    target: f32,
    hysteresis: f32,
) -> Result<(), Error> {
    let mut rt = runtime::Builder::new()
        .basic_scheduler()
        .enable_all()
        .build()
        .expect("Could not construct runtime");
    let local = tokio::task::LocalSet::new();
    let (tx, rx) = channel(false);

    local.spawn_local(
        tokio::net::TcpStream::connect(sensor)
            .map_err(Error::from)
            .and_then(move |s| async move {
                if let Err(e) = s.set_nodelay(true) {
                    eprintln!("Warning: could not set nodelay ({})", e)
                };
                let (reader, _) = split(s);
                let mut messages =
                    capnp_futures::ReadStream::new(reader.compat(), Default::default());

                let mut on = false;
                while let Some(msg) = messages.next().await {
                    let temperature = msg?.get_root::<sensor_capnp::state::Reader>()?.get_value();
                    update(&mut on, temperature, target, hysteresis);
                    tx.broadcast(on).map_err(Error::from)?;
                }
                Ok(())
            }),
    );

    let listener = listen_on(port)?;
    let listener = rt.enter(|| tokio::net::TcpListener::from_std(listener))?;

    local.block_on(
        &mut rt,
        listener
            .map_err(Error::from)
            .try_for_each_concurrent(None, |s| async {
                if let Err(e) = s.set_nodelay(true) {
                    eprintln!("Warning: could not set nodelay ({})", e)
                };

                let (mut reader, writer) = split(s);
                let mut rx = rx.clone();

                if let Some(msg) = read_message((&mut reader).compat(), Default::default()).await? {
                    msg.get_root::<controller_capnp::hello::Reader>()?
                        .get_type()?;
                } else {
                    return Ok(());
                }

                let network = capnp_rpc::twoparty::VatNetwork::new(
                    reader.compat(),
                    writer.compat_write(),
                    capnp_rpc::rpc_twoparty_capnp::Side::Client,
                    Default::default(),
                );

                let mut rpc_system = capnp_rpc::RpcSystem::new(Box::new(network), None);
                let actor: actor_capnp::actor::Client =
                    rpc_system.bootstrap(capnp_rpc::rpc_twoparty_capnp::Side::Server);

                local.spawn_local(rpc_system.map_err(|e| eprintln!("RPC error ({})", e)));

                while let Some(on) = rx.next().await {
                    let mut req = actor.toggle_request();
                    req.get().set_state(on);
                    req.send().promise.await?.get()?;
                }
                Ok(())
            }),
    )
}
