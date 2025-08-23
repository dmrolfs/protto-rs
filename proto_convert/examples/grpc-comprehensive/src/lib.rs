pub mod proto {
    tonic::include_proto!("service");
}

#[cfg(feature = "meta-file")]
pub mod proto_metadata {
    include!(concat!(env!("OUT_DIR"), "/proto_field_metadata.rs"));

    pub fn print_all_metadata() {
        for (message, fields) in get_all_metadata() {
            println!("Message: {}", message);
            for (field, optional) in *fields {
                println!("  {}: optional={}", field, optional);
            }
        }
    }
}

mod basic_types;
mod complex_types;
mod default_types;
mod error_types;
mod shared_types;

#[cfg(test)]
mod advanced_tests;
#[cfg(test)]
mod basic_tests;
#[cfg(test)]
mod default_tests;
#[cfg(test)]
mod edge_case_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod integration_tests;
