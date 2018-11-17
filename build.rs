extern crate capnpc;

fn main() {
    let schemas = ["temperature.capnp", "actor.capnp"];

    for schema in schemas.iter() {
        capnpc::CompilerCommand::new()
            .src_prefix("schema")
            .file(format!("schema/{}", schema))
            .run().expect("Schema compiler command failed");
    }
}
