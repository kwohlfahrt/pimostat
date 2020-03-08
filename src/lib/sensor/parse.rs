use std::io::{self, BufRead};

#[derive(Debug)]
pub enum Error {
    Checksum,
    Parse,
    IO(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IO(err)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(_: std::num::ParseIntError) -> Error {
        Error::Parse
    }
}

pub fn parse<R: BufRead>(r: &mut R) -> Result<f32, Error> {
    let mut s = String::with_capacity(40);
    r.read_line(&mut s)?;
    if !s.ends_with("YES\n") {
        return Err(Error::Checksum);
    };

    r.read_line(&mut s)?;
    let t = s.rsplit("t=").next().ok_or(Error::Parse)?.trim();
    Ok(t.parse::<i32>()? as f32 / 1000.0)
}

#[cfg(test)]
mod test {
    use super::*;

    const COLD: &str = concat!(
        "a3 01 4b 46 7f ff 0e 10 d8 : crc=d8 YES\n",
        "a3 01 4b 46 7f ff 0e 10 d8 t=10234\n",
    );
    const HOT: &str = concat!(
        "a3 01 4b 46 7f ff 0e 10 d8 : crc=d8 YES\n",
        "a3 01 4b 46 7f ff 0e 10 d8 t=32768\n",
    );
    const INVALID: &str = concat!(
        "a3 01 4b 46 7f ff 0e 10 d8 : crc=d8 NO\n",
        "a3 01 4b 46 7f ff 0e 10 d8 t=10234\n",
    );

    #[test]
    fn sample_parse() {
        assert_eq!(parse(&mut HOT.as_bytes()).unwrap(), 32.768);
        assert_eq!(parse(&mut COLD.as_bytes()).unwrap(), 10.234);
    }

    #[test]
    fn error() {
        if let Err(Error::Checksum) = parse(&mut INVALID.as_bytes()) {
        } else {
            panic!("Invalid data did not cause error");
        }
    }
}
