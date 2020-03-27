pub fn split_host_port(address: &str) -> (&str, u16) {
    let (host, port) = address.split_at(address.rfind(":").expect("Expected address in the form 'host:port'"));
    let host = host.trim_start_matches("[").trim_end_matches("]");
    let port = port.trim_start_matches(":");
    (host, port.parse().expect("Invalid port"))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_ipv6() {
	assert_eq!(split_host_port("localhost:5000"), ("localhost", 5000));
	assert_eq!(split_host_port("[::1]:5000"), ("::1", 5000));
    }
}
