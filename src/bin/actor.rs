use std::path::{Path, PathBuf};
use std::convert::TryFrom;

use clap::Parser;

use pimostat::actor::{run, GpioActor};
use pimostat::error::Error;
use pimostat::util::split_host_port;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    // TODO: Parse controller with clap
    controller: String,
    chip: PathBuf,
    line: u32,

    #[clap(long)]
    no_tls: bool,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let chip: &Path = &args.chip;

    run(
        split_host_port(&args.controller),
        args.no_tls,
        GpioActor::try_from((chip, args.line))?,
    )
}
