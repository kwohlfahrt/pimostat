pub mod parse;

use std::fs::{read, File};
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use futures::future::{pending, ready, select, FutureExt, TryFutureExt};
use futures::pin_mut;
use futures::stream::{StreamExt, TryStreamExt};
use tokio::io::{split, AsyncRead, AsyncWrite};
use tokio::runtime;
use tokio::sync::{oneshot, watch};
use tokio_util::compat::Tokio02AsyncWriteCompatExt;

use crate::error::Error;
use crate::sensor_capnp;
use crate::socket::listen_on;
use parse::parse;

async fn handle_connection<S>(s: S, mut rx: watch::Receiver<f32>) -> Result<(), Error>
where
    S: AsyncRead + AsyncWrite,
{
    let (_, writer) = split(s);
    let mut writer = writer.compat_write();

    while rx.changed().await.is_ok() {
        let mut msg_builder = capnp::message::Builder::new_default();
        {
            let mut msg = msg_builder.init_root::<sensor_capnp::state::Builder>();
            msg.set_value(*rx.borrow());
        }

        capnp_futures::serialize::write_message(&mut writer, msg_builder).await?;
    }
    Ok(())
}

pub fn run(
    address: Option<(&str, u16)>,
    cert: Option<&Path>,
    source: &Path,
    interval: u32,
    termination: Option<oneshot::Receiver<()>>,
) -> Result<(), Error> {
    let tls_acceptor = cert
        .map(|cert| -> Result<_, Error> {
            let identity = native_tls::Identity::from_pkcs12(&read(cert)?, "")?;
            Ok(tokio_native_tls::TlsAcceptor::from(
                native_tls::TlsAcceptor::new(identity)?,
            ))
        })
        .transpose()?;
    let termination = termination.map(|rx| rx.or_else(|_| pending()));

    let rt = runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("Could not construct runtime");

    let mut source = BufReader::new(File::open(source)?);
    let (tx, rx) = watch::channel(parse(&mut source)?);

    let interval = {
        let _guard = rt.enter();
        tokio::time::interval(Duration::from_secs(interval as u64))
            .map(move |_| {
                source.seek(SeekFrom::Start(0))?;
                Ok(parse(&mut source)?)
            })
            .try_for_each(|value| ready(tx.send(value)).err_into())
    };

    let listener = listen_on(address)?;
    let listener = {
        let _guard = rt.enter();
        tokio::net::TcpListener::from_std(listener)?.err_into()
    };

    if let Some(tls_acceptor) = tls_acceptor {
        let listener = listener
            .and_then(|s| tls_acceptor.accept(s).err_into())
            .try_for_each_concurrent(None, |s| handle_connection(s, rx.clone()));
        pin_mut!(listener);
        // TODO: Look into take_until for this
        if let Some(termination) = termination {
            let interval = select(interval, termination).map(|e| e.factor_first().0);
            rt.block_on(select(interval, listener)).factor_first().0
        } else {
            rt.block_on(select(interval, listener)).factor_first().0
        }
    } else {
        let listener = listener.try_for_each_concurrent(None, |s| handle_connection(s, rx.clone()));
        pin_mut!(listener);
        if let Some(termination) = termination {
            let interval = select(interval, termination).map(|e| e.factor_first().0);
            rt.block_on(select(interval, listener)).factor_first().0
        } else {
            rt.block_on(select(interval, listener)).factor_first().0
        }
    }
}
