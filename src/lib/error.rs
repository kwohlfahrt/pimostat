extern crate capnp;
extern crate futures;
extern crate tokio;

#[derive(Debug)]
pub enum Error {
    CapnP(capnp::Error),
    Schema(capnp::NotInSchema),
    Timer(tokio::time::Error),
    IO(std::io::Error),
    Send(futures::channel::mpsc::SendError),
    Checksum,
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Error::IO(e) => write!(fmt, "IO({})", e),
            Error::CapnP(e) => write!(fmt, "Schema({})", e),
            Error::Schema(e) => write!(fmt, "CapnP({})", e),
            Error::Send(e) => write!(fmt, "Send({})", e),
            Error::Timer(e) => write!(fmt, "Timer({})", e),
            Error::Checksum => write!(fmt, "Checksum"),
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

impl From<super::sensor::parse::Error> for Error {
    fn from(e: super::sensor::parse::Error) -> Self {
        match e {
            super::sensor::parse::Error::ChecksumError => Error::Checksum,
            super::sensor::parse::Error::IO(e) => Error::IO(e),
        }
    }
}

impl std::error::Error for Error {}
