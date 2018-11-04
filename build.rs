extern crate capnpc;

fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/temperature.capnp")
        .run().expect("Schema compiler command failed")
}
