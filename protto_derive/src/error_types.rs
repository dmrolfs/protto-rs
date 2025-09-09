use super::*;

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

/// Determines the effective error type for a field
pub fn get_effective_error_type(
    proto_meta: &attribute_parser::ProtoFieldMeta,
    struct_level_error_type: &Option<syn::Type>,
) -> Option<syn::Type> {
    if let Some(field_error_type) = &proto_meta.error_type {
        return Some(
            syn::parse_str(field_error_type).expect("Failed to parse field-level error_type"),
        );
    }

    struct_level_error_type.clone()
}

/// Determines the actual error type to use in trait implementations
pub fn get_actual_error_type(
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
pub fn generate_conversion_error_enum(struct_name: &syn::Ident) -> proc_macro2::TokenStream {
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
pub fn generate_error_conversions(error_name: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        impl From<String> for #error_name {
            fn from(err: String) -> Self {
                Self::MissingField(err)
            }
        }
    }
}
