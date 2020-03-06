extern crate capnp;
extern crate capnp_futures;
extern crate capnp_rpc;

use capnp_rpc::pry;

extern crate tokio;
use tokio::io::split;
use tokio::runtime;
extern crate tokio_util;
use tokio_util::compat::{Tokio02AsyncReadCompatExt, Tokio02AsyncWriteCompatExt};

extern crate futures;
use futures::TryFutureExt;

use crate::{actor_capnp, controller_capnp, error::Error};

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::net::SocketAddr;

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

pub fn run(addr: SocketAddr, gpio: &str) -> Result<(), Error> {
    let gpio = OpenOptions::new()
        .read(false)
        .write(true)
        .open(gpio)
        .expect("Could not open GPIO file");

    let mut rt = runtime::Builder::new()
        .basic_scheduler()
        .enable_all()
        .build()
        .expect("Could not construct runtime");

    let client =
        actor_capnp::actor::ToClient::new(Actor { gpio }).into_client::<capnp_rpc::Server>();

    let mut builder = capnp::message::Builder::new_default();
    {
        let mut msg = builder.init_root::<controller_capnp::hello::Builder>();
        msg.set_type(controller_capnp::hello::Type::Actor);
    }

    rt.block_on(async {
        let s = tokio::net::TcpStream::connect(&addr).await?;
        if let Err(e) = s.set_nodelay(true) {
            eprintln!("Warning: could not set nodelay ({})", e)
        };
        let (reader, mut writer) = split(s);

        capnp_futures::serialize::write_message((&mut writer).compat_write(), builder)
            .map_err(Error::CapnP)
            .await?;

        let network = capnp_rpc::twoparty::VatNetwork::new(
            reader.compat(),
            writer.compat_write(),
            capnp_rpc::rpc_twoparty_capnp::Side::Server,
            Default::default(),
        );

        capnp_rpc::RpcSystem::new(Box::new(network), Some(client.client))
            .map_err(Error::CapnP)
            .await
    })
}
