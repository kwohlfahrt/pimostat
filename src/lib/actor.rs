extern crate capnp;
extern crate capnp_futures;
extern crate capnp_rpc;
extern crate futures;
extern crate tokio;
extern crate tokio_tls;
extern crate tokio_util;

use std::env;
use std::fs::{read, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use capnp_rpc::pry;
use futures::TryFutureExt;
use tokio::io::{split, AsyncRead, AsyncWrite};
use tokio::runtime;
use tokio_util::compat::{Tokio02AsyncReadCompatExt, Tokio02AsyncWriteCompatExt};

use crate::error::Error;
use crate::{actor_capnp, controller_capnp};

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

async fn run_rpc<S>(s: S, client: actor_capnp::actor::Client) -> Result<(), Error>
where
    S: AsyncRead + AsyncWrite + 'static,
{
    let mut builder = capnp::message::Builder::new_default();
    {
        let mut msg = builder.init_root::<controller_capnp::hello::Builder>();
        msg.set_type(controller_capnp::hello::Type::Actor);
    }

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
}

pub fn run(controller: (&str, u16), tls: bool, gpio: &Path) -> Result<(), Error> {
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

    let controller_host = controller.0;
    let controller = tokio::net::TcpStream::connect(&controller)
        .map_err(Error::from)
        .inspect_ok(|s| {
            if let Err(e) = s.set_nodelay(true) {
                eprintln!("Warning: could not set nodelay ({})", e)
            };
        });

    if tls {
        let mut builder = native_tls::TlsConnector::builder();
        // For testing. rust-native-tls does not respect this env var on its own
        if let Some(cert) = env::var("SSL_CERT_FILE").ok() {
            builder.add_root_certificate(native_tls::Certificate::from_pem(&read(cert)?).unwrap());
        };
        let controller = controller.and_then(|s| async move {
            let connector = tokio_tls::TlsConnector::from(builder.build()?);
            Ok(connector.connect(controller_host, s).await?)
        });

        rt.block_on(controller.and_then(|s| run_rpc(s, client)))
    } else {
        rt.block_on(controller.and_then(|s| run_rpc(s, client)))
    }
}
