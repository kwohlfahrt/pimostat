extern crate capnp;
extern crate capnp_rpc;
extern crate futures;
extern crate tokio;
extern crate tokio_util;

pub mod parse;

use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use futures::future::ok;
use futures::stream::{StreamExt, TryStreamExt};
use tokio::io::split;
use tokio::runtime;
use tokio::sync::watch::channel;
use tokio_util::compat::Tokio02AsyncWriteCompatExt;

use crate::error::Error;
use crate::sensor_capnp;
use crate::socket::listen_on;
use parse::parse;

pub fn run(port: Option<u16>, source: &Path, interval: u32) -> Result<(), Error> {
    let listener = listen_on(port)?;

    let mut rt = runtime::Builder::new()
        .basic_scheduler()
        .enable_all()
        .build()
        .expect("Could not construct runtime");

    let mut source = BufReader::new(File::open(source)?);
    let (tx, rx) = channel(parse(&mut source)?);

    let interval = rt.enter(|| tokio::time::interval(Duration::from_secs(interval as u64)));
    let listener = rt.enter(|| tokio::net::TcpListener::from_std(listener))?;

    rt.spawn(
        interval
            .map(move |_| {
                source.seek(SeekFrom::Start(0))?;
                let value = parse(&mut source).map_err(Error::from)?;
                tx.broadcast(value).map_err(Error::from)
            })
            .try_for_each(ok),
    );

    rt.block_on(listener.map_err(Error::from).try_for_each(|s| {
        let (_, mut writer) = split(s);
        let mut rx = rx.clone();

        tokio::spawn(async move {
            while let Some(value) = rx.next().await {
                let mut msg_builder = capnp::message::Builder::new_default();
                {
                    let mut msg = msg_builder.init_root::<sensor_capnp::state::Builder>();
                    msg.set_value(value);
                }

                if let Err(e) = capnp_futures::serialize::write_message(
                    (&mut writer).compat_write(),
                    msg_builder,
                )
                .await
                {
                    eprintln!("Could not send message ({})", e);
                    break;
                }
            }
        });
        ok(())
    }))
}
