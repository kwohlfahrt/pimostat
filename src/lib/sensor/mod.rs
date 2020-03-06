extern crate capnp;
extern crate capnp_rpc;
extern crate tokio;
extern crate tokio_util;

mod parse;

use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use tokio::io::split;
use tokio::runtime;
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

    rt.block_on(async {
        let mut listener = tokio::net::TcpListener::from_std(listener)?;

        loop {
            let (s, _) = listener.accept().await?;
            let (_, mut writer) = split(s);
            // Inefficient, we open the file for each incoming stream
            let mut source = BufReader::new(File::open(source)?);
            let mut interval = tokio::time::interval(Duration::from_secs(interval as u64));

            tokio::spawn(async move {
                loop {
                    interval.tick().await;

                    let mut msg_builder = capnp::message::Builder::new_default();
                    {
                        let mut msg = msg_builder.init_root::<sensor_capnp::state::Builder>();
                        source.seek(SeekFrom::Start(0)).unwrap();
                        msg.set_value(parse(&mut source).unwrap());
                    }

                    if let Err(e) = capnp_futures::serialize::write_message(
                        (&mut writer).compat_write(),
                        msg_builder,
                    )
                    .await
                    {
                        eprintln!("Could not send message ({})", e);
                        break;
                    };
                }
            });
        }
    })
}
