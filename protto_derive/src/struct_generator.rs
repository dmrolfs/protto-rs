use crate::analysis::error_analysis;
use crate::debug::CallStackDebug;
use crate::field::{self, FieldProcessingContext};
use quote::quote;
use std::collections::HashSet;

#[allow(unused)]
pub struct StructImplConfig<'a> {
    pub name: &'a syn::Ident,
    pub fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    pub proto_module: &'a str,
    pub proto_name: &'a str,
    pub proto_path: &'a syn::Path,
    pub struct_level_error_type: &'a Option<syn::Type>,
    pub struct_level_error_fn: &'a Option<String>,
    pub proto_ignored_fields: &'a HashSet<String>,
}

pub fn generate_struct_implementations(config: StructImplConfig) -> proc_macro2::TokenStream {
    let proto_path = &config.proto_path;

    let _trace = CallStackDebug::with_context(
        "struct_generator",
        "generate_struct_implementations",
        config.name,
        "",
        &[
            ("proto_module", config.proto_module),
            ("proto_name", config.proto_name),
            ("proto_path", &quote! { #proto_path }.to_string()),
            (
                "struct_level_error_type",
                &config
                    .struct_level_error_type
                    .as_ref()
                    .map(|et| quote! { #et }.to_string())
                    .unwrap_or_else(|| "none".to_string()),
            ),
            (
                "struct_level_error_fn",
                &config
                    .struct_level_error_fn
                    .clone()
                    .unwrap_or_else(|| "none".to_string()),
            ),
            (
                "proto_ignored_fields",
                &config
                    .proto_ignored_fields
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ],
    );

    let struct_name = config.name;
    let fields = config.fields;

    let (conversion_error_def, error_conversions, needs_try_from) =
        generate_error_definitions_if_needed(struct_name, fields, config.struct_level_error_type);

    let actual_error_type = get_actual_error_type(
        needs_try_from,
        config.struct_level_error_type,
        &default_error_name(struct_name),
    );

    let error_name = default_error_name(struct_name);

    let proto_ignored_fields = config.proto_ignored_fields;

    // Generate bidirectional conversions in single pass
    let mut field_conversions = Vec::new();
    let mut conversion_errors = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();

        let _trace = CallStackDebug::with_context(
            "struct_impl",
            "generate_struct_implementations",
            config.name,
            field_name,
            &[],
        );

        let ctx = FieldProcessingContext::new(
            struct_name,
            field,
            &error_name,
            config.struct_level_error_type,
            config.struct_level_error_fn,
            config.proto_module,
            config.proto_name,
        );

        match field::generate_bidirectional_field_conversion(field, &ctx) {
            Ok((proto_to_rust, rust_to_proto)) => {
                field_conversions.push((field_name, proto_to_rust, rust_to_proto));
            }
            Err(error_msg) => {
                conversion_errors.push((field_name, error_msg));
            }
        }
    }

    // Handle any conversion errors
    if !conversion_errors.is_empty() {
        let error_msgs: Vec<String> = conversion_errors
            .iter()
            .map(|(field_name, error)| format!("Field '{}': {}", field_name, error))
            .collect();
        let combined_error = error_msgs.join("\n");
        return quote! { compile_error!(#combined_error); };
    }

    // Generate From and Into implementations
    let proto_to_rust_fields: Vec<_> = field_conversions
        .iter()
        .map(|(_, proto_to_rust, _)| proto_to_rust)
        .filter(|ts| !ts.is_empty())
        .collect();
    let rust_to_proto_fields: Vec<_> = field_conversions
        .iter()
        .map(|(_, _, rust_to_proto)| rust_to_proto)
        .filter(|ts| !ts.is_empty())
        .collect();

    let proto_ignore_defaults = generate_proto_ignore_defaults(proto_ignored_fields);

    let proto_type_path = format!("{}::{}", config.proto_module, config.proto_name);
    let proto_type: syn::Path = syn::parse_str(&proto_type_path).unwrap();

    let from_trait_impl = if needs_try_from {
        quote! {
            impl TryFrom<#proto_type> for #struct_name {
                type Error = #actual_error_type;

                fn try_from(proto_struct: #proto_type) -> Result<Self, Self::Error> {
                    Ok(Self {
                        #(#proto_to_rust_fields,)*
                    })
                }
            }
        }
    } else {
        quote! {
            impl From<#proto_type> for #struct_name {
                fn from(proto_struct: #proto_type) -> Self {
                    Self {
                        #(#proto_to_rust_fields,)*
                    }
                }
            }
        }
    };

    let into_trait_impl = quote! {
        impl Into<#proto_type> for #struct_name {
            fn into(self) -> #proto_type {
                let my_struct = self;
                #proto_type {
                    #(#rust_to_proto_fields,)*
                    #(#proto_ignore_defaults,)*
                }
            }
        }
    };

    quote! {
        #conversion_error_def
        #error_conversions
        #from_trait_impl
        #into_trait_impl
    }
}

/// Main orchestration function for generating all error-related definitions
fn generate_error_definitions_if_needed(
    name: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    struct_level_error_type: &Option<syn::Type>,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, bool) {
    let requirements = error_analysis::analyze_error_requirements(fields, struct_level_error_type);

    let conversion_error_def = if requirements.needs_try_from
        && requirements.needs_default_error
        && struct_level_error_type.is_none()
    {
        generate_conversion_error_enum(name)
    } else {
        quote! {}
    };

    let error_conversions = if requirements.needs_error_conversions {
        let error_name = default_error_name(name);
        generate_error_conversions(&error_name)
    } else {
        quote! {}
    };

    (
        conversion_error_def,
        error_conversions,
        requirements.needs_try_from,
    )
}

/// Generate Default::default() assignments for `ignore` fields
fn generate_proto_ignore_defaults(
    proto_ignored_fields: &HashSet<String>,
) -> Vec<proc_macro2::TokenStream> {
    proto_ignored_fields
        .iter()
        .map(|field_name| {
            let field_ident: syn::Ident = syn::parse_str(field_name)
                .unwrap_or_else(|_| panic!("Invalid field name in proto_ignore: '{}'", field_name));

            quote! {
                #field_ident: Default::default()
            }
        })
        .collect()
}

/// Generates the default error name for a struct
pub fn default_error_name(struct_name: &syn::Ident) -> syn::Ident {
    syn::Ident::new(
        &format!(
            "{struct_name}{}",
            crate::constants::DEFAULT_CONVERSION_ERROR_SUFFIX
        ),
        struct_name.span(),
    )
}

/// Determines the actual error type to use in trait implementations
fn get_actual_error_type(
    needs_try_from: bool,
    struct_level_error_type: &Option<syn::Type>,
    error_name: &syn::Ident,
) -> syn::Type {
    if needs_try_from {
        struct_level_error_type.clone().unwrap_or_else(|| {
            syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path::from(error_name.clone()),
            })
        })
    } else {
        syn::parse_str("String").unwrap()
    }
}

/// Generates the conversion error enum definition
fn generate_conversion_error_enum(struct_name: &syn::Ident) -> proc_macro2::TokenStream {
    let error_name = default_error_name(struct_name);

    quote! {
        #[derive(Debug, Clone, PartialEq)]
        pub enum #error_name {
            MissingField(String),
        }

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::MissingField(field) => write!(f, "Missing required field: {field}"),
                }
            }
        }

        impl std::error::Error for #error_name {}
    }
}

/// Generates error conversion implementations
fn generate_error_conversions(error_name: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        impl From<String> for #error_name {
            fn from(err: String) -> Self {
                Self::MissingField(err)
            }
        }
    }
}
