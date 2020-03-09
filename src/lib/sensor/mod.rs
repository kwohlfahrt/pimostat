extern crate capnp;
extern crate capnp_rpc;
extern crate futures;
extern crate native_tls;
extern crate tokio;
extern crate tokio_tls;
extern crate tokio_util;

pub mod parse;

use std::fs::{read, File};
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use futures::future::{join, ok, TryFutureExt};
use futures::stream::{StreamExt, TryStreamExt};
use tokio::io::{split, AsyncRead, AsyncWrite};
use tokio::runtime;
use tokio::sync::watch::channel;
use tokio_util::compat::Tokio02AsyncWriteCompatExt;

use crate::error::Error;
use crate::sensor_capnp;
use crate::socket::listen_on;
use parse::parse;

async fn handle_connection<S, R>(s: S, mut rx: R) -> Result<(), Error>
where
    S: AsyncRead + AsyncWrite,
    R: StreamExt<Item = f32> + std::marker::Unpin,
{
    let (_, mut writer) = split(s);

    while let Some(value) = rx.next().await {
        let mut msg_builder = capnp::message::Builder::new_default();
        {
            let mut msg = msg_builder.init_root::<sensor_capnp::state::Builder>();
            msg.set_value(value);
        }

        capnp_futures::serialize::write_message((&mut writer).compat_write(), msg_builder).await?;
    }
    Ok(())
}

pub fn run(
    port: Option<u16>,
    source: &Path,
    interval: u32,
    cert: Option<&Path>,
) -> Result<(), Error> {
    let listener = listen_on(port)?;

    let mut rt = runtime::Builder::new()
        .basic_scheduler()
        .enable_all()
        .build()
        .expect("Could not construct runtime");

    let mut source = BufReader::new(File::open(source)?);
    let (tx, rx) = channel(parse(&mut source)?);

    let interval = rt
        .enter(|| tokio::time::interval(Duration::from_secs(interval as u64)))
        .skip(1)
        .map(move |_| {
            source.seek(SeekFrom::Start(0))?;
            let value = parse(&mut source).map_err(Error::from)?;
            tx.broadcast(value).map_err(Error::from)
        })
        .try_for_each(ok);

    let listener = rt
        .enter(|| tokio::net::TcpListener::from_std(listener))?
        .map_err(Error::from);
    if let Some(cert) = cert {
        let identity = native_tls::Identity::from_pkcs12(&read(cert)?, "")?;
        let acceptor = tokio_tls::TlsAcceptor::from(native_tls::TlsAcceptor::new(identity)?);
        let listener = listener
            .and_then(|s| acceptor.accept(s).map_err(Error::from))
            .try_for_each_concurrent(None, |s| handle_connection(s, rx.clone()));
        let (interval, listener) = rt.block_on(join(interval, listener));
        interval.or(listener)
    } else {
        let listener = listener.try_for_each_concurrent(None, |s| handle_connection(s, rx.clone()));
        let (interval, listener) = rt.block_on(join(interval, listener));
        interval.or(listener)
    }
}
