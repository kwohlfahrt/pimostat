use std::env;
use std::fs::read;

use capnp_rpc::pry;
use futures::TryFutureExt;
use tokio::io::{split, AsyncRead, AsyncWrite};
use tokio::runtime;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

mod file;
mod gpio;

pub use file::FileActor;
pub use gpio::GpioActor;

use crate::error::Error;
use crate::{actor_capnp, controller_capnp};

pub trait Actor {
    fn update(&mut self, state: bool) -> std::io::Result<()>;
}

impl<A: Actor> actor_capnp::actor::Server for A {
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

    let (reader, writer) = split(s);
    let mut writer = writer.compat_write();

    capnp_futures::serialize::write_message(&mut writer, builder)
        .map_err(Error::CapnP)
        .await?;

    let network = capnp_rpc::twoparty::VatNetwork::new(
        reader.compat(),
        writer,
        capnp_rpc::rpc_twoparty_capnp::Side::Server,
        Default::default(),
    );

    capnp_rpc::RpcSystem::new(Box::new(network), Some(client.client))
        .map_err(Error::CapnP)
        .await
}

pub fn run<A>(controller: (&str, u16), tls: bool, actor: A) -> Result<(), Error>
where
    A: Actor + 'static,
{
    let (tls_host, _) = controller;
    let tls_connector = if tls {
        let mut builder = native_tls::TlsConnector::builder();
        // For testing. rust-native-tls does not respect this env var on its own
        if let Some(cert) = env::var("SSL_CERT_FILE").ok() {
            builder.add_root_certificate(native_tls::Certificate::from_pem(&read(cert)?).unwrap());
        };
        Some(tokio_native_tls::TlsConnector::from(builder.build()?))
    } else {
        None
    };

    let rt = runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("Could not construct runtime");

    let client = capnp_rpc::new_client(actor);

    let controller = tokio::net::TcpStream::connect(&controller)
        .err_into()
        .inspect_ok(|s| {
            if let Err(e) = s.set_nodelay(true) {
                eprintln!("Warning: could not set nodelay ({})", e)
            };
        });

    if let Some(tls_connector) = tls_connector {
        let controller =
            controller.and_then(|s| async move { Ok(tls_connector.connect(tls_host, s).await?) });

        rt.block_on(controller.and_then(|s| run_rpc(s, client)))
    } else {
        rt.block_on(controller.and_then(|s| run_rpc(s, client)))
    }
}
