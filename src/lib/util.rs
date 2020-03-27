pub fn split_host_port(address: &str) -> (&str, u16) {
    let (host, port) = address.split_at(
        address
            .rfind(":")
            .expect("Expected address in the form 'host:port'"),
    );
    (host, port.parse().expect("Invalid port"))
}
