use std::path::PathBuf;

use clap::Parser;

use pimostat::controller::run;
use pimostat::error::Error;
use pimostat::util::split_host_port;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    // TODO: Parse sensor/address with clap
    sensor: String,
    temperature: f32,
    address: Option<String>,

    #[clap(short, long, default_value_t = 1.5)]
    hysteresis: f32,

    #[clap(short, long, parse(from_os_str))]
    certificate: Option<PathBuf>,

    #[clap(long)]
    no_tls: bool,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let address = args.address.as_ref().map(|a| split_host_port(a));

    run(
        address,
        args.certificate.as_ref(),
        split_host_port(&args.sensor),
        args.no_tls,
        args.temperature,
        args.hysteresis,
    )
}
