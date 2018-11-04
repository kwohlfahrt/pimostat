extern crate capnp;

include!(concat!(env!("OUT_DIR"), "/temperature_capnp.rs"));

fn main() {
    let mut builder = capnp::message::Builder::new_default();

    {
        let mut temperature_msg = builder.init_root::<temperature::Builder>();
        temperature_msg.set_value(21.5);
    }

    let mut buffer = Vec::new();
    capnp::serialize::write_message(&mut buffer, &builder).unwrap();

    let deserialized = capnp::serialize::read_message(
        &mut buffer.as_slice(),
        capnp::message::ReaderOptions::new()
    ).unwrap();

    let temperature_reader = deserialized.get_root::<temperature::Reader>().unwrap();

    println!("Temperature is: {}", temperature_reader.get_value());
}
