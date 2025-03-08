use glob::glob;
use std::{io::Result, path::Path};

fn main() -> Result<()> {
    let proto_files = glob("proto/*.proto")
        .unwrap()
        .map(|res| res.unwrap().into_boxed_path())
        .collect::<Vec<Box<Path>>>();

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&proto_files, &["proto/"])?;
    Ok(())
}
