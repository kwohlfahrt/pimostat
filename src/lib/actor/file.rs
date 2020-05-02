use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use super::Actor;

pub struct FileActor {
    handle: File,
}

impl Actor for FileActor {
    fn update(&mut self, state: bool) -> std::io::Result<()> {
        write!(self.handle, "{}", if state { "0" } else { "1" })?;
        self.handle.flush()?;
        Ok(())
    }
}

impl TryFrom<&Path> for FileActor {
    type Error = std::io::Error;

    fn try_from(src: &Path) -> Result<Self, Self::Error> {
        let handle = OpenOptions::new().read(false).write(true).open(src)?;
        Ok(Self { handle })
    }
}
