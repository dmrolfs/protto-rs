use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/");

    let proto_files = generate_prost_protos()?;

    if !proto_files.is_empty() {
        println!(
            "cargo:warning=Generating metadata for {} proto files",
            proto_files.len()
        );
    } else {
        println!("cargo:warning=No proto files found for metadata generation");
    }

    Ok(())
}

fn generate_prost_protos() -> Result<Vec<Box<Path>>, Box<dyn std::error::Error>> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let manifest_path = Path::new(&manifest_dir);
    let proto_dir = manifest_path.join("proto");
    if !proto_dir.exists() {
        println!(
            "cargo:warning=Proto directory {:?} does not exist",
            proto_dir
        );
        return Ok(Vec::new());
    }

    // println!("cargo:info=Generating prost proto .rs to {:?}", proto_dir);

    let proto_pattern = proto_dir.join("*.proto");
    let proto_files = glob::glob(proto_pattern.to_str().unwrap())?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|path| path.into_boxed_path())
        .collect::<Vec<Box<Path>>>();

    if proto_files.is_empty() {
        println!("cargo:warning=No .proto files found in {:?}", proto_dir);
        return Ok(Vec::new());
    }

    println!("cargo:warning=Found proto files: {:?}", proto_files);

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .type_attribute(
            "service.Header",
            "#[cfg_attr(test, derive(proptest_derive::Arbitrary))]",
        )
        .type_attribute(
            "service.Request",
            "#[cfg_attr(test, derive(proptest_derive::Arbitrary))]",
        )
        .type_attribute(
            "service.Track",
            "#[cfg_attr(test, derive(proptest_derive::Arbitrary))]",
        )
        .type_attribute(
            "service.State",
            "#[cfg_attr(test, derive(proptest_derive::Arbitrary))]",
        )
        .type_attribute(
            "service.HasOptional",
            "#[cfg_attr(test, derive(proptest_derive::Arbitrary))]",
        )
        .type_attribute(
            "service.TrackWithOptionals",
            "#[cfg_attr(test, derive(proptest_derive::Arbitrary))]",
        )
        .type_attribute(
            "service.MixedBehaviorTrack",
            "#[cfg_attr(test, derive(proptest_derive::Arbitrary))]",
        )
        .type_attribute(
            "service.HasStraight",
            "#[cfg_attr(test, derive(proptest_derive::Arbitrary))]",
        )
        .compile_protos(&proto_files, &[proto_dir])?;

    Ok(proto_files)
}
