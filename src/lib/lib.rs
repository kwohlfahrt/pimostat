pub mod actor_capnp {
    include!(concat!(env!("OUT_DIR"), "/actor_capnp.rs"));
}

pub mod sensor_capnp {
    include!(concat!(env!("OUT_DIR"), "/sensor_capnp.rs"));
}

pub mod controller_capnp {
    include!(concat!(env!("OUT_DIR"), "/controller_capnp.rs"));
}

pub mod error;

mod systemd;
pub use systemd::get_systemd_socket;
