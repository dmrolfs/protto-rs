use std::env;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = Path::new(&manifest_dir);
    let proto_dir = manifest_path.join("proto");
    let proto_files = glob::glob(proto_dir.join("*.proto").to_str().unwrap())
        .unwrap()
        .map(|res| res.unwrap().into_boxed_path())
        .collect::<Vec<Box<Path>>>();

    dbg!("manifest dir {}", manifest_dir);
    dbg!("proto files {:?}", &proto_files);
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
        .compile_protos(&proto_files, &[proto_dir])
        .unwrap();
}
