#![cfg(test)]

pub mod proto {
    tonic::include_proto!("service");
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
