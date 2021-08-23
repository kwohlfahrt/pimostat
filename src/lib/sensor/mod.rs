pub mod parse;

use std::fs::{read, File};
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use futures::future::{pending, ready, select, Either, TryFutureExt};
use futures::io::AsyncWrite;
use futures::pin_mut;
use futures::stream::{StreamExt, TryStreamExt};
use tokio::net::TcpListener;
use tokio::runtime;
use tokio::sync::{oneshot, watch};
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

use crate::error::Error;
use crate::sensor_capnp;
use crate::socket::listen_on;
use parse::parse;

async fn handle_connection<W: AsyncWrite + Unpin>(
    mut writer: W,
    mut rx: watch::Receiver<f32>,
) -> Result<(), Error> {
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
    let termination = termination.map_or(Either::Left(pending()), |rx| {
        Either::Right(rx.or_else(|_| pending::<Result<_, Error>>()))
    });

    let rt = runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("Could not construct runtime");

    let mut source = BufReader::new(File::open(source)?);
    let (tx, rx) = watch::channel(parse(&mut source)?);

    let interval = {
        let _guard = rt.enter();
        IntervalStream::new(tokio::time::interval(Duration::from_secs(interval as u64)))
            .map(move |_| {
                source.seek(SeekFrom::Start(0))?;
                Ok(parse(&mut source)?)
            })
            .take_until(termination)
            .try_for_each(|value| ready(tx.send(value)).err_into())
    };

    let listener = listen_on(address)?;
    let listener = {
        let _guard = rt.enter();
        TcpListenerStream::new(TcpListener::from_std(listener)?).err_into()
    }
    .and_then(|s| async {
        let writer = if let Some(ref tls_acceptor) = tls_acceptor {
            Either::Left(tls_acceptor.accept(s).await?.compat_write())
        } else {
            Either::Right(s.compat_write())
        };
        Ok(writer)
    });

    let listener = listener.try_for_each_concurrent(None, |s| handle_connection(s, rx.clone()));
    pin_mut!(listener);
    rt.block_on(select(interval, listener)).factor_first().0
}
