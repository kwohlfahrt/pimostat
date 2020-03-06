use std::io::{self, BufRead};

#[derive(Debug)]
pub enum Error {
    ChecksumError,
    IO(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IO(err)
    }
}

pub fn parse<R: BufRead>(r: &mut R) -> Result<f32, Error> {
    let mut s = String::with_capacity(40);
    r.read_line(&mut s)?;
    if !s.ends_with("YES\n") {
        return Err(Error::ChecksumError);
    };

    r.read_line(&mut s)?;
    let t = s.rsplit("t=").next().unwrap().trim();
    Ok(t.parse::<i32>().unwrap() as f32 / 1000.0)
}

#[cfg(test)]
mod test {
    use super::*;

    const SAMPLE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/w1_therm"));

    #[test]
    fn sample_parse() {
        assert_eq!(parse(&mut SAMPLE.as_bytes()).unwrap(), 32.768);
    }
}
