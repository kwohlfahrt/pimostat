use std::env;
use std::fs::read;
use std::path::Path;

use capnp_futures::serialize::read_message;
use futures::future::select;
use futures::pin_mut;
use futures::{StreamExt, TryFuture, TryFutureExt, TryStream, TryStreamExt};
use tokio::io::{split, AsyncRead, AsyncWrite};
use tokio::runtime;
use tokio::sync::watch;
use tokio_util::compat::{Tokio02AsyncReadCompatExt, Tokio02AsyncWriteCompatExt};

use crate::error::Error;
use crate::socket::listen_on;
use crate::{actor_capnp, controller_capnp, sensor_capnp};

fn update(on: &mut bool, temperature: f32, target: f32, hysteresis: f32) {
    if temperature > target {
        *on = false;
    } else if temperature < (target - hysteresis) {
        *on = true;
    }
}

fn run_controller<S, L>(
    sensor: impl TryFuture<Ok = S, Error = Error>,
    listener: impl TryStream<Ok = L, Error = Error>,
    mut rt: tokio::runtime::Runtime,
    target: f32,
    hysteresis: f32,
) -> Result<(), Error>
where
    S: AsyncRead + Unpin + 'static,
    L: AsyncRead + AsyncWrite + 'static,
{
    let local = tokio::task::LocalSet::new();
    let (tx, rx) = watch::channel(false);

    let server = listener
        .err_into()
        .try_for_each_concurrent(None, |s| async {
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
        });
    pin_mut!(server);

    let sensor = sensor.and_then(|s| async {
        let mut messages = capnp_futures::ReadStream::new(s.compat(), Default::default());

        let mut on = false;
        while let Some(msg) = messages.next().await {
            let temperature = msg?.get_root::<sensor_capnp::state::Reader>()?.get_value();
            update(&mut on, temperature, target, hysteresis);
            tx.broadcast(on)?;
        }
        Ok(())
    });
    pin_mut!(sensor);

    local
        .block_on(&mut rt, select(sensor, server))
        .factor_first()
        .0
}

pub fn run(
    address: Option<(&str, u16)>,
    cert: Option<&Path>,
    sensor: (&str, u16),
    tls: bool,
    target: f32,
    hysteresis: f32,
) -> Result<(), Error> {
    let (tls_host, _) = sensor;
    let tls_connector = if tls {
        let mut builder = native_tls::TlsConnector::builder();
        // For testing. rust-native-tls does not respect this env var on its own
        if let Some(cert) = env::var("SSL_CERT_FILE").ok() {
            builder.add_root_certificate(native_tls::Certificate::from_pem(&read(cert)?).unwrap());
        };
        Some(tokio_tls::TlsConnector::from(builder.build()?))
    } else {
        None
    };
    let tls_acceptor = cert
        .map(|cert| -> Result<_, Error> {
            let identity = native_tls::Identity::from_pkcs12(&read(cert)?, "")?;
            Ok(tokio_tls::TlsAcceptor::from(native_tls::TlsAcceptor::new(
                identity,
            )?))
        })
        .transpose()?;

    let rt = runtime::Builder::new()
        .basic_scheduler()
        .enable_all()
        .build()
        .expect("Could not construct runtime");

    let listener = listen_on(address)?;
    let listener = rt
        .enter(|| tokio::net::TcpListener::from_std(listener))?
        .err_into()
        .inspect_ok(|s| {
            if let Err(e) = s.set_nodelay(true) {
                eprintln!("Warning: could not set nodelay ({})", e)
            };
        });
    let sensor = tokio::net::TcpStream::connect(sensor)
        .err_into()
        .inspect_ok(|s| {
            if let Err(e) = s.set_nodelay(true) {
                eprintln!("Warning: could not set nodelay ({})", e)
            };
        });
    pin_mut!(sensor);

    if let Some(tls_acceptor) = tls_acceptor {
        let listener = listener.and_then(|s| tls_acceptor.accept(s).err_into());

        if let Some(tls_connector) = tls_connector {
            let sensor =
                sensor.and_then(|s| async move { Ok(tls_connector.connect(tls_host, s).await?) });
            run_controller(sensor, listener, rt, target, hysteresis)
        } else {
            run_controller(sensor, listener, rt, target, hysteresis)
        }
    } else {
        if let Some(tls_connector) = tls_connector {
            let sensor =
                sensor.and_then(|s| async move { Ok(tls_connector.connect(tls_host, s).await?) });
            run_controller(sensor, listener, rt, target, hysteresis)
        } else {
            run_controller(sensor, listener, rt, target, hysteresis)
        }
    }
}
