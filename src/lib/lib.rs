extern crate capnp;

#[derive(Debug)]
pub enum Error{
    CapnP(capnp::Error),
    IO(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Error::IO(e) => write!(fmt, "IO({})", e),
            Error::CapnP(e) => write!(fmt, "CapnP({})", e),
        }
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

pub mod actor_capnp {
    include!(concat!(env!("OUT_DIR"), "/actor_capnp.rs"));
}

pub mod sensor_capnp {
    include!(concat!(env!("OUT_DIR"), "/sensor_capnp.rs"));
}

pub mod controller_capnp {
    include!(concat!(env!("OUT_DIR"), "/controller_capnp.rs"));
}
