use crate::error::mode::ErrorMode;
use crate::analysis::{
    field_analysis::FieldProcessingContext,
    type_analysis,
};
use crate::conversion::custom_strategy::CustomConversionStrategy;
use crate::field::conversion_strategy::{
    CollectionStrategy, DirectStrategy, FieldConversionStrategy, OptionStrategy,
};
use quote::quote;

impl FieldConversionStrategy {
    /// Generate proto->rust conversion code using new simplified logic
    pub fn generate_proto_to_rust_conversion(
        &self,
        ctx: &FieldProcessingContext,
    ) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field = &ctx.proto_field_ident;

        match self {
            Self::Ignore => generate_ignore_proto_to_rust(ctx),

            Self::Custom(custom_strategy) => {
                generate_custom_proto_to_rust(custom_strategy, field_name, proto_field)
            }

            Self::Direct(direct_strategy) => {
                generate_direct_proto_to_rust(direct_strategy, field_name, proto_field)
            }

            Self::Option(option_strategy) => {
                generate_option_proto_to_rust(option_strategy, field_name, proto_field)
            }

            Self::Transparent(error_mode) => generate_transparent_proto_to_rust(error_mode, ctx),

            Self::Collection(collection_strategy) => {
                generate_collection_proto_to_rust(collection_strategy, ctx)
            }
        }
    }

    /// Generate rust->proto conversion code using new simplified logic
    pub fn generate_rust_to_proto_conversion(
        &self,
        ctx: &FieldProcessingContext,
    ) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field = &ctx.proto_field_ident;

        match self {
            Self::Ignore => {
                // Ignored fields are not included in proto struct
                quote! { /* field ignored */ }
            }

            Self::Custom(custom_strategy) => {
                generate_custom_rust_to_proto(custom_strategy, field_name, proto_field)
            }

            Self::Direct(direct_strategy) => {
                generate_direct_rust_to_proto(direct_strategy, field_name, proto_field)
            }

            Self::Option(option_strategy) => {
                generate_option_rust_to_proto(option_strategy, field_name, proto_field)
            }

            Self::Transparent(error_mode) => {
                generate_transparent_rust_to_proto(error_mode, field_name, proto_field)
            }

            Self::Collection(collection_strategy) => {
                generate_collection_rust_to_proto(collection_strategy, field_name, proto_field)
            }
        }
    }
}

// -- Proto-to-Rust generation functions --
fn generate_ignore_proto_to_rust(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;

    if let Some(default_fn_name) = &ctx.default_fn {
        let default_fn_path: syn::Path =
            syn::parse_str(default_fn_name).expect("Failed to parse default_fn function path");
        quote! { #field_name: #default_fn_path() }
    } else {
        quote! { #field_name: Default::default() }
    }
}

fn generate_custom_proto_to_rust(
    custom_strategy: &CustomConversionStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    match custom_strategy {
        CustomConversionStrategy::FromFn(fn_path)
        | CustomConversionStrategy::Bidirectional(fn_path, _) => {
            let from_fn: syn::Path =
                syn::parse_str(fn_path).expect("Failed to parse function path");
            quote! { #field_name: #from_fn(proto_struct.#proto_field) }
        }
        CustomConversionStrategy::IntoFn(_) => {
            // Fallback to .into() for proto->rust when only rust->proto function provided
            quote! { #field_name: proto_struct.#proto_field.into() }
        }
    }
}

fn generate_direct_proto_to_rust(
    direct_strategy: &DirectStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    match direct_strategy {
        DirectStrategy::Assignment => {
            quote! { #field_name: proto_struct.#proto_field }
        }
        DirectStrategy::WithConversion => {
            quote! { #field_name: proto_struct.#proto_field.into() }
        }
    }
}

fn generate_option_proto_to_rust(
    option_strategy: &OptionStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    match option_strategy {
        OptionStrategy::Wrap => {
            quote! { #field_name: Some(proto_struct.#proto_field.into()) }
        }
        OptionStrategy::Unwrap(error_mode) => {
            generate_unwrap_with_error_mode(error_mode, field_name, proto_field)
        }
        OptionStrategy::Map => {
            quote! { #field_name: proto_struct.#proto_field.map(|v| v.into()) }
        }
    }
}

fn generate_transparent_proto_to_rust(
    error_mode: &ErrorMode,
    ctx: &FieldProcessingContext,
) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field = &ctx.proto_field_ident;
    let field_type = ctx.field_type;

    match error_mode {
        ErrorMode::None | ErrorMode::Panic => {
            quote! {
                #field_name: #field_type::from(
                    proto_struct.#proto_field
                        .expect(&format!(
                            "Proto field {} is required for transparent conversion",
                            stringify!(#proto_field)
                        ))
                )
            }
        }
        ErrorMode::Error => {
            // Generate error handling code - this would need access to error type/function
            quote! {
                #field_name: #field_type::from(
                    proto_struct.#proto_field
                        .ok_or_else(|| ConversionError::MissingField(stringify!(#proto_field).to_string()))?
                )
            }
        }
        ErrorMode::Default(default_fn) => {
            if let Some(fn_name) = default_fn {
                let default_fn_path: syn::Path =
                    syn::parse_str(fn_name).expect("Failed to parse default function");
                quote! {
                    #field_name: #field_type::from(
                        proto_struct.#proto_field.unwrap_or_else(|| {
                            let default_val: #field_type = #default_fn_path();
                            default_val.into()
                        })
                    )
                }
            } else {
                quote! {
                    #field_name: #field_type::from(
                        proto_struct.#proto_field.unwrap_or_default()
                    )
                }
            }
        }
    }
}

fn generate_collection_proto_to_rust(
    collection_strategy: &CollectionStrategy,
    ctx: &FieldProcessingContext,
) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field = &ctx.proto_field_ident;

    match collection_strategy {
        CollectionStrategy::Collect(error_mode) => {
            match error_mode {
                ErrorMode::Default(Some(default_fn)) => {
                    let default_fn_path: syn::Path =
                        syn::parse_str(default_fn).expect("Failed to parse default function");
                    quote! {
                        #field_name: if proto_struct.#proto_field.is_empty() {
                            #default_fn_path()
                        } else {
                            proto_struct.#proto_field.into_iter().map(Into::into).collect()
                        }
                    }
                }
                ErrorMode::Default(None) => {
                    quote! {
                        #field_name: if proto_struct.#proto_field.is_empty() {
                            Default::default()
                        } else {
                            proto_struct.#proto_field.into_iter().map(Into::into).collect()
                        }
                    }
                }
                ErrorMode::Error => {
                    // Could generate error handling for empty collections if needed
                    quote! {
                        #field_name: proto_struct.#proto_field.into_iter().map(Into::into).collect()
                    }
                }
                _ => {
                    quote! {
                        #field_name: proto_struct.#proto_field.into_iter().map(Into::into).collect()
                    }
                }
            }
        }
        CollectionStrategy::MapOption => {
            // Check if rust field is Option<Vec<T>> -> handle empty vec as None
            if is_option_vec_type(ctx.field_type) {
                quote! {
                    #field_name: if proto_struct.#proto_field.is_empty() {
                        None
                    } else {
                        Some(proto_struct.#proto_field.into_iter().map(Into::into).collect())
                    }
                }
            } else {
                // Option<Vec<T>> case where we map the option
                quote! {
                    #field_name: proto_struct.#proto_field
                        .map(|vec| vec.into_iter().map(Into::into).collect())
                }
            }
        }
        CollectionStrategy::DirectAssignment => {
            quote! { #field_name: proto_struct.#proto_field }
        }
    }
}

// -- Rust-to-Proto generation functions --
fn generate_custom_rust_to_proto(
    custom_strategy: &CustomConversionStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    match custom_strategy {
        CustomConversionStrategy::IntoFn(fn_path)
        | CustomConversionStrategy::Bidirectional(_, fn_path) => {
            let into_fn: syn::Path =
                syn::parse_str(fn_path).expect("Failed to parse function path");
            quote! { #proto_field: #into_fn(my_struct.#field_name) }
        }
        CustomConversionStrategy::FromFn(_) => {
            // Fallback to .into() for rust->proto when only proto->rust function provided
            quote! { #proto_field: my_struct.#field_name.into() }
        }
    }
}

fn generate_direct_rust_to_proto(
    direct_strategy: &DirectStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    match direct_strategy {
        DirectStrategy::Assignment => {
            quote! { #proto_field: my_struct.#field_name }
        }
        DirectStrategy::WithConversion => {
            quote! { #proto_field: my_struct.#field_name.into() }
        }
    }
}

fn generate_option_rust_to_proto(
    option_strategy: &OptionStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    match option_strategy {
        OptionStrategy::Wrap => {
            quote! { #proto_field: Some(my_struct.#field_name.into()) }
        }
        OptionStrategy::Unwrap(_) => {
            // For rust->proto, unwrap the option
            quote! { #proto_field: my_struct.#field_name.unwrap_or_default().into() }
        }
        OptionStrategy::Map => {
            quote! { #proto_field: my_struct.#field_name.map(|v| v.into()) }
        }
    }
}

fn generate_transparent_rust_to_proto(
    _error_mode: &ErrorMode,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    // For rust->proto, transparent always uses Into conversion
    quote! { #proto_field: my_struct.#field_name.into() }
}

fn generate_collection_rust_to_proto(
    collection_strategy: &CollectionStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    match collection_strategy {
        CollectionStrategy::Collect(_) => {
            quote! {
                #proto_field: my_struct.#field_name.into_iter().map(Into::into).collect()
            }
        }
        CollectionStrategy::MapOption => {
            quote! {
                #proto_field: my_struct.#field_name
                    .map(|vec| vec.into_iter().map(Into::into).collect())
                    .unwrap_or_default()
            }
        }
        CollectionStrategy::DirectAssignment => {
            quote! { #proto_field: my_struct.#field_name }
        }
    }
}

// -- Helper functions --
fn generate_unwrap_with_error_mode(
    error_mode: &ErrorMode,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    match error_mode {
        ErrorMode::None | ErrorMode::Panic => {
            quote! {
                #field_name: proto_struct.#proto_field
                    .expect(&format!("Proto field {} is required", stringify!(#proto_field)))
                    .into()
            }
        }
        ErrorMode::Error => {
            quote! {
                #field_name: proto_struct.#proto_field
                    .ok_or_else(|| ValidationError::missing_field(stringify!(#proto_field)))?
                    .into()
            }
        }
        ErrorMode::Default(Some(default_fn)) => {
            let default_fn_path: syn::Path =
                syn::parse_str(default_fn).expect("Failed to parse default function");
            quote! {
                #field_name: proto_struct.#proto_field
                    .map(|v| v.into())
                    .unwrap_or_else(|| #default_fn_path())
            }
        }
        ErrorMode::Default(None) => {
            quote! {
                #field_name: proto_struct.#proto_field
                    .map(|v| v.into())
                    .unwrap_or_default()
            }
        }
    }
}

fn is_option_vec_type(field_type: &syn::Type) -> bool {
    type_analysis::get_inner_type_from_option(field_type)
        .map(|inner| type_analysis::is_vec_type(&inner))
        .unwrap_or(false)
}

// Replace the placeholder implementation in compatibility testing
#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::compatibility::test_helpers;

    #[test]
    fn test_code_generation_produces_valid_tokens() {
        // Test that all strategy types produce valid TokenStream output
        let test_cases = vec![
            ("ignore_field", FieldConversionStrategy::Ignore),
            (
                "direct_assignment",
                FieldConversionStrategy::Direct(DirectStrategy::Assignment),
            ),
            (
                "direct_conversion",
                FieldConversionStrategy::Direct(DirectStrategy::WithConversion),
            ),
        ];

        for (name, strategy) in test_cases {
            let (field, context) = test_helpers::create_mock_context(
                "TestStruct",
                "test_field",
                "String",
                "proto",
                &[],
            );

            let proto_to_rust = strategy.generate_proto_to_rust_conversion(&context);
            let rust_to_proto = strategy.generate_rust_to_proto_conversion(&context);

            // Verify tokens are not empty and parse correctly
            assert!(
                !proto_to_rust.is_empty(),
                "Proto->Rust generation failed for {}",
                name
            );
            assert!(
                !rust_to_proto.is_empty() || matches!(strategy, FieldConversionStrategy::Ignore),
                "Rust->Proto generation failed for {}",
                name
            );

            // Verify the generated code contains the field name
            let proto_to_rust_str = proto_to_rust.to_string();
            let rust_to_proto_str = rust_to_proto.to_string();

            assert!(
                proto_to_rust_str.contains("test_field"),
                "Generated proto->rust code should contain field name: {}",
                proto_to_rust_str
            );

            if !matches!(strategy, FieldConversionStrategy::Ignore) {
                assert!(
                    rust_to_proto_str.contains("test_field"),
                    "Generated rust->proto code should contain field name: {}",
                    rust_to_proto_str
                );
            }
        }
    }

    #[test]
    fn test_error_mode_code_generation() {
        let error_modes = vec![
            ErrorMode::None,
            ErrorMode::Panic,
            ErrorMode::Default(None),
            ErrorMode::Default(Some("test_default".to_string())),
        ];

        for error_mode in error_modes {
            let strategy = FieldConversionStrategy::Transparent(error_mode.clone());
            let (field, context) = test_helpers::create_mock_context(
                "TestStruct",
                "test_field",
                "TransparentWrapper",
                "proto",
                &["transparent"],
            );

            let proto_to_rust = strategy.generate_proto_to_rust_conversion(&context);
            let code_str = proto_to_rust.to_string();

            match &error_mode {
                ErrorMode::Panic => {
                    assert!(code_str.contains("expect"), "Panic mode should use expect")
                }
                ErrorMode::Default(Some(_)) => assert!(
                    code_str.contains("test_default"),
                    "Should use custom default"
                ),
                ErrorMode::Default(None) => assert!(
                    code_str.contains("unwrap_or_default"),
                    "Should use default trait"
                ),
                _ => {} // Other modes have different patterns
            }
        }
    }

    #[test]
    fn test_custom_strategy_code_generation() {
        let custom_strategies = vec![
            CustomConversionStrategy::FromFn("custom_from".to_string()),
            CustomConversionStrategy::IntoFn("custom_into".to_string()),
            CustomConversionStrategy::Bidirectional(
                "custom_from".to_string(),
                "custom_into".to_string(),
            ),
        ];

        for custom_strategy in custom_strategies {
            let strategy = FieldConversionStrategy::Custom(custom_strategy.clone());
            let (field, context) = test_helpers::create_mock_context(
                "TestStruct",
                "test_field",
                "CustomType",
                "proto",
                &[],
            );

            let proto_to_rust = strategy.generate_proto_to_rust_conversion(&context);
            let rust_to_proto = strategy.generate_rust_to_proto_conversion(&context);

            let proto_to_rust_str = proto_to_rust.to_string();
            let rust_to_proto_str = rust_to_proto.to_string();

            match &custom_strategy {
                CustomConversionStrategy::FromFn(fn_name) => {
                    assert!(
                        proto_to_rust_str.contains(fn_name),
                        "Should use custom from function"
                    );
                }
                CustomConversionStrategy::IntoFn(fn_name) => {
                    assert!(
                        rust_to_proto_str.contains(fn_name),
                        "Should use custom into function"
                    );
                }
                CustomConversionStrategy::Bidirectional(from_fn, into_fn) => {
                    assert!(
                        proto_to_rust_str.contains(from_fn),
                        "Should use custom from function"
                    );
                    assert!(
                        rust_to_proto_str.contains(into_fn),
                        "Should use custom into function"
                    );
                }
            }
        }
    }
}
