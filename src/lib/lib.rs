extern crate capnp;
extern crate tokio;
extern crate futures;

use std::os::unix::io::FromRawFd;

#[derive(Debug)]
pub enum Error {
    CapnP(capnp::Error),
    Schema(capnp::NotInSchema),
    Timer(tokio::time::Error),
    IO(std::io::Error),
    Send(futures::channel::mpsc::SendError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Error::IO(e) => write!(fmt, "IO({})", e),
            Error::CapnP(e) => write!(fmt, "Schema({})", e),
            Error::Schema(e) => write!(fmt, "CapnP({})", e),
            Error::Send(e) => write!(fmt, "Send({})", e),
            Error::Timer(e) => write!(fmt, "Timer({})", e),
        }
    }
}

impl From<capnp::NotInSchema> for Error {
    fn from(e: capnp::NotInSchema) -> Self {
        Error::Schema(e)
    }
}

impl From<capnp::Error> for Error {
    fn from(e: capnp::Error) -> Self {
        Error::CapnP(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO(e)
    }
}

impl std::error::Error for Error {}

pub fn get_systemd_socket() -> std::net::TcpListener {
    let listen_pid = std::env::var("LISTEN_PID")
	.map(
	    |pid| pid.parse::<u32>().expect("Invalid LISTEN_PID")
	)
	.expect("LISTEN_PID is not set");
    let listen_fds = std::env::var("LISTEN_FDS")
	.map(
	    |fd| fd.parse::<u32>().expect("Invalid LISTEN_FDS")
	)
	.expect("LISTEN_FDS is not set");
    if listen_pid != std::process::id() {
	panic!("LISTEN_PID does not match current PID");
    }
    if listen_fds != 1 {
	panic!("LISTEN_FDS is not 1");
    }
    unsafe { std::net::TcpListener::from_raw_fd(3) }
}

pub mod actor_capnp {
    include!(concat!(env!("OUT_DIR"), "/actor_capnp.rs"));
}

pub mod sensor_capnp {
    include!(concat!(env!("OUT_DIR"), "/sensor_capnp.rs"));
}

pub mod controller_capnp {
    include!(concat!(env!("OUT_DIR"), "/controller_capnp.rs"));
}
