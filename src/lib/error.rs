extern crate capnp;
extern crate native_tls;
extern crate tokio;

#[derive(Debug)]
pub enum Error {
    CapnP(capnp::Error),
    Schema(capnp::NotInSchema),
    Timer(tokio::time::Error),
    IO(std::io::Error),
    Send,
    Checksum,
    Parse,
    Tls(native_tls::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Error::IO(e) => write!(fmt, "IO({})", e),
            Error::CapnP(e) => write!(fmt, "Schema({})", e),
            Error::Schema(e) => write!(fmt, "CapnP({})", e),
            Error::Send => write!(fmt, "Send"),
            Error::Timer(e) => write!(fmt, "Timer({})", e),
            Error::Checksum => write!(fmt, "Checksum"),
            Error::Parse => write!(fmt, "Parse"),
            Error::Tls(e) => write!(fmt, "TLS({})", e),
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

impl From<native_tls::Error> for Error {
    fn from(e: native_tls::Error) -> Self {
        Error::Tls(e)
    }
}

impl<T> From<tokio::sync::watch::error::SendError<T>> for Error {
    fn from(_: tokio::sync::watch::error::SendError<T>) -> Self {
        Error::Send
    }
}

impl From<super::sensor::parse::Error> for Error {
    fn from(e: super::sensor::parse::Error) -> Self {
        match e {
            super::sensor::parse::Error::Checksum => Error::Checksum,
            super::sensor::parse::Error::Parse => Error::Parse,
            super::sensor::parse::Error::IO(e) => Error::IO(e),
        }
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(_: std::num::ParseIntError) -> Self {
        Error::Parse
    }
}

impl std::error::Error for Error {}
