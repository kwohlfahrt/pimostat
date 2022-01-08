
use std::path::PathBuf;

use clap::Parser;

use pimostat::error::Error;
use pimostat::sensor::run;
use pimostat::util::split_host_port;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    source: PathBuf,
    address: Option<String>, //TODO: Parse address with clap

    #[clap(short, long, default_value_t = 60)]
    interval: u32,

    #[clap(short, long, parse(from_os_str))]
    certificate: Option<PathBuf>,
}


fn main() -> Result<(), Error> {
    let args = Args::parse();
    let address = args.address.as_ref().map(|a| split_host_port(a));

    run(address, args.certificate.as_deref(), &args.source, args.interval, None)
}
