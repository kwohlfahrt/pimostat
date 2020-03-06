extern crate capnp;
extern crate capnp_rpc;

extern crate clap;
use clap::{App, Arg};

extern crate tokio;
use tokio::io::split;
use tokio::runtime;

extern crate tokio_util;
use tokio_util::compat::Tokio02AsyncWriteCompatExt;

extern crate pimostat;
use pimostat::{error::Error, get_systemd_socket, sensor_capnp};

use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::net::SocketAddr;
use std::time::Duration;

mod parse;
use parse::parse;

fn main() -> Result<(), Error> {
    let matches = App::new("Thermostat Sensor")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true),
        )
        .arg(Arg::with_name("source").required(true))
        .arg(Arg::with_name("interval").required(true))
        .get_matches();

    let port: Option<u16> = matches
        .value_of("port")
        .map(|p| p.parse().expect("Invalid port"));
    let listener = match port {
        None => Ok(get_systemd_socket()),
        Some(p) => {
            let addrs = [
                SocketAddr::new("0.0.0.0".parse().unwrap(), p),
                SocketAddr::new("::".parse().unwrap(), p),
            ];
            std::net::TcpListener::bind(&addrs[..])
        }
    }?;

    let mut rt = runtime::Builder::new()
        .basic_scheduler()
        .enable_all()
        .build()
        .expect("Could not construct runtime");

    let interval: u32 = matches.value_of("interval").unwrap().parse().unwrap();

    rt.block_on(async {
        let mut listener = tokio::net::TcpListener::from_std(listener)?;

        loop {
            let (s, _) = listener.accept().await?;
            let (_, mut writer) = split(s);
            // Inefficient, we open the file for each incoming stream
            let mut source =
                BufReader::new(File::open(matches.value_of("source").unwrap()).unwrap());
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
