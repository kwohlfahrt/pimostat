extern crate gpiochip;

use std::convert::TryFrom;
use std::path::Path;

use gpiochip::{GpioChip, GpioHandle};

use super::Actor;

pub struct GpioActor {
    handle: GpioHandle,
}

impl Actor for GpioActor {
    fn update(&mut self, state: bool) -> std::io::Result<()> {
        self.handle.set(if state { 1 } else { 0 })?;
        Ok(())
    }
}

impl TryFrom<(&Path, u32)> for GpioActor {
    type Error = std::io::Error;

    fn try_from(src: (&Path, u32)) -> Result<Self, Self::Error> {
        let (path, line) = src;
        let gpio = GpioChip::new(path)?;
        let handle = gpio.request("thermostat_relay", gpiochip::RequestFlags::OUTPUT, line, 0)?;
        Ok(Self { handle })
    }
}
