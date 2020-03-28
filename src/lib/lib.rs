pub mod actor_capnp {
    include!(concat!(env!("OUT_DIR"), "/actor_capnp.rs"));
}

pub mod controller_capnp {
    include!(concat!(env!("OUT_DIR"), "/controller_capnp.rs"));
}

pub mod sensor_capnp {
    include!(concat!(env!("OUT_DIR"), "/sensor_capnp.rs"));
}

mod socket;

pub mod actor;
pub mod controller;
pub mod error;
pub mod sensor;
pub mod util;
