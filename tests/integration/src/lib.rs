#![cfg(test)]

pub mod proto {
    tonic::include_proto!("service");
}

mod basic_types;
mod complex_types;
mod default_types;
mod error_types;
mod shared_types;

mod additional_edge_case_tests;
#[cfg(test)]
mod advanced_tests;
mod attribute_parser_tests;
#[cfg(test)]
mod basic_tests;
mod boolean_boundary_tests;
mod boundary_property_tests;
mod code_generation_edge_tests;
#[cfg(test)]
mod default_tests;
#[cfg(test)]
mod edge_case_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod integration_tests;
mod strategy_selection_tests;
mod type_inference_edge_tests;
