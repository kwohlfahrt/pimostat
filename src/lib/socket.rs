use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener};
use std::os::unix::io::FromRawFd;

pub fn get_systemd_socket() -> std::net::TcpListener {
    let listen_pid = std::env::var("LISTEN_PID")
        .map(|pid| pid.parse::<u32>().expect("Invalid LISTEN_PID"))
        .expect("LISTEN_PID is not set");
    let listen_fds = std::env::var("LISTEN_FDS")
        .map(|fd| fd.parse::<u32>().expect("Invalid LISTEN_FDS"))
        .expect("LISTEN_FDS is not set");
    if listen_pid != std::process::id() {
        panic!("LISTEN_PID does not match current PID");
    }
    if listen_fds != 1 {
        panic!("LISTEN_FDS is not 1");
    }
    unsafe { std::net::TcpListener::from_raw_fd(3) }
}

pub fn listen_on(port: Option<u16>) -> Result<TcpListener, std::io::Error> {
    match port {
        None => Ok(get_systemd_socket()),
        Some(p) => {
            let addrs = [
                SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), p),
                SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), p),
            ];
            TcpListener::bind(&addrs[..])
        }
    }
}
