use crate::analysis::macro_input::ParsedInput;
use crate::debug::CallStackDebug;
use proc_macro::TokenStream;
use syn::{self, DeriveInput};

mod constants {
    pub const PRIMITIVE_TYPES: &[&str] =
        &["i32", "u32", "i64", "u64", "f32", "f64", "bool", "String"];
    pub const DEFAULT_PROTO_MODULE: &str = "proto";

    pub const PROTTO_ATTRIBUTE: &str = "protto";
    pub const DEFAULT_CONVERSION_ERROR_SUFFIX: &str = "ConversionError";
    pub const USE_DEFAULT_IMPL: &str = "Default::default";
}

mod analysis;
mod debug;
mod enum_generator;
mod field;
mod struct_generator;
mod tuple_generator;

mod utils {
    /// Converts a PascalCase string to SCREAMING_SNAKE_CASE.
    ///
    /// Inserts `_` before each uppercase letter (except the first). This handles
    /// standard PascalCase correctly but does NOT handle consecutive uppercase
    /// (acronyms): `HTTPStatus` becomes `H_T_T_P_STATUS`, not `HTTP_STATUS`.
    /// For acronym-aware conversion, consider the `heck` crate.
    pub fn to_screaming_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() && i != 0 {
                result.push('_');
            }
            result.push(c.to_ascii_uppercase());
        }
        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn single_word() {
            assert_eq!(to_screaming_snake_case("Status"), "STATUS");
        }

        #[test]
        fn two_words() {
            assert_eq!(
                to_screaming_snake_case("SubscriptionTier"),
                "SUBSCRIPTION_TIER"
            );
        }

        #[test]
        fn three_words() {
            assert_eq!(to_screaming_snake_case("PaymentMethod"), "PAYMENT_METHOD");
        }

        #[test]
        fn already_uppercase_single() {
            assert_eq!(to_screaming_snake_case("OK"), "O_K");
        }

        #[test]
        fn single_variant() {
            assert_eq!(to_screaming_snake_case("Free"), "FREE");
        }

        #[test]
        fn multi_word_variant() {
            assert_eq!(to_screaming_snake_case("CreditCard"), "CREDIT_CARD");
        }

        /// Documents known limitation: consecutive uppercase chars each get a separator.
        #[test]
        fn consecutive_uppercase_known_limitation() {
            // HTTPStatus → H_T_T_P_STATUS (not HTTP_STATUS)
            // This is a known limitation; use `heck` crate for acronym-aware conversion.
            assert_eq!(to_screaming_snake_case("HTTPStatus"), "H_T_T_P_STATUS");
        }
    }
}

mod registry {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};

    /// Global registry for tracking enum types across macro invocations
    static ENUM_TYPE_REGISTRY: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

    /// Initialize the global enum registry
    fn get_enum_registry() -> &'static Mutex<HashSet<String>> {
        ENUM_TYPE_REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
    }

    /// Register a type as an enum (called during enum processing)
    pub fn register_enum_type(type_name: &str) {
        if let Ok(mut registry) = get_enum_registry().lock() {
            registry.insert(type_name.to_string());
        }
    }

    /// Check if a type is registered as an enum (called during field processing)
    pub fn is_registered_enum_type(type_name: &str) -> bool {
        get_enum_registry()
            .lock()
            .map(|registry| registry.contains(type_name))
            .unwrap_or(false)
    }
}

#[proc_macro_derive(Protto, attributes(protto))]
pub fn protto_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let parsed_input = ParsedInput::new(ast.clone());

    let name = parsed_input.name;

    let _trace = CallStackDebug::new("protto_derive::lib", "protto_derive", &name, "");

    // -- phase 1 - check if this is an enum type with #[proto(enum)] --
    if let syn::Data::Enum(_) = &ast.data {
        registry::register_enum_type(&ast.ident.to_string())
    }

    // -- phase 2 - process the struct/enum --
    let generated = match &ast.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields_named) => {
                let config = struct_generator::StructImplConfig {
                    name: &name,
                    fields: &fields_named.named,
                    proto_module: &parsed_input.proto_module,
                    proto_name: &parsed_input.proto_name,
                    proto_path: &parsed_input.proto_path,
                    struct_level_error_type: &parsed_input.struct_level_error_type,
                    struct_level_error_fn: &parsed_input.struct_level_error_fn,
                    proto_ignored_fields: &parsed_input.proto_ignored_fields,
                };

                struct_generator::generate_struct_implementations(config)
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                tuple_generator::generate_tuple_implementations(&name, fields_unnamed)
            }
            syn::Fields::Unit => {
                panic!("Protto does not support unit structs");
            }
        },
        syn::Data::Enum(data_enum) => {
            let variants = &data_enum.variants;
            enum_generator::generate_enum_conversions(&name, variants, &parsed_input.proto_module)
        }
        _ => panic!("Protto only supports structs and enums, not unions"),
    };

    _trace.generated_code(&generated, name, "", "bidirectional_proto_to_rust", &[]);

    generated.into()
}
