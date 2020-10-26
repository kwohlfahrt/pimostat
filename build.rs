fn main() {
    let schemas = ["sensor.capnp", "actor.capnp", "controller.capnp"];

    for schema in schemas.iter() {
        capnpc::CompilerCommand::new()
            .src_prefix("schema")
            .file(format!("schema/{}", schema))
            .run()
            .expect("Schema compiler command failed");
    }
}
