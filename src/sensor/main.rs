extern crate capnp;
extern crate capnp_rpc;

extern crate futures;
use futures::{Future, Stream};

extern crate tokio;
use tokio::io::AsyncRead;
// Capn'p clients are not Sync
use tokio::runtime::current_thread;

extern crate clap;
use clap::{App, Arg};

extern crate pimostat;
use pimostat::{get_systemd_socket, sensor_capnp, Error};

use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::net::SocketAddr;
use std::time::Duration;

mod parse;
use parse::parse;

fn main() -> Result<(), std::io::Error> {
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
                SocketAddr::new("::0".parse().unwrap(), p),
            ];
            std::net::TcpListener::bind(&addrs[..])
        }
    }?;

    let interval: u32 = matches.value_of("interval").unwrap().parse().unwrap();

    let incoming = tokio::net::TcpListener::from_std(listener, &tokio::reactor::Handle::default())?
        .incoming()
        .map_err(Error::IO)
        .and_then(|s| {
            let (_, writer) = s.split();

            // Inefficient, we open the file for each incoming stream
            let mut source =
                BufReader::new(File::open(matches.value_of("source").unwrap()).unwrap());
            tokio::timer::Interval::new_interval(Duration::from_secs(interval as u64))
                .map_err(Error::Timer)
                .fold(writer, move |writer, _| {
                    let mut msg_builder = capnp::message::Builder::new_default();
                    {
                        let mut msg = msg_builder.init_root::<sensor_capnp::state::Builder>();
                        source.seek(SeekFrom::Start(0)).unwrap();
                        msg.set_value(parse(&mut source).unwrap());
                    }
                    capnp_futures::serialize::write_message(writer, msg_builder)
                        .map_err(Error::CapnP)
                        .map(|(writer, _)| writer)
                })
        })
        .for_each(|_| Ok(()));

    current_thread::block_on_all(incoming).expect("Failed to run RPC server");
    Ok(())
}
