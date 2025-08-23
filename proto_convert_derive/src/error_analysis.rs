use super::*;
use crate::expect_analysis::ExpectMode;

/// Analyzes fields to determine if TryFrom trait is needed
pub fn requires_try_from(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> bool {
    fields.iter().any(|field| {
        if attribute_parser::has_proto_ignore(field) {
            false
        } else {
            let proto_meta = attribute_parser::ProtoFieldMeta::from_field(field).unwrap_or_default();
            let expect_mode = ExpectMode::from_field_meta(field, &proto_meta);
            matches!(expect_mode, ExpectMode::Error)
        }
    })
}

/// Analyzes fields to determine if default error type generation is needed
pub fn requires_default_error_type(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    struct_level_error_type: &Option<syn::Type>,
) -> bool {
    fields.iter().any(|field| {
        if attribute_parser::has_proto_ignore(field) {
            return false;
        }
        let proto_meta = attribute_parser::ProtoFieldMeta::from_field(field).unwrap_or_default();
        if matches!(ExpectMode::from_field_meta(field, &proto_meta), ExpectMode::Error) {
            let effective_error_type = error_types::get_effective_error_type(&proto_meta, struct_level_error_type);
            effective_error_type.is_none()
        } else {
            false
        }
    })
}

/// Comprehensive analysis of error requirements for a struct
pub struct ErrorRequirements {
    pub needs_try_from: bool,
    pub needs_default_error: bool,
    pub needs_error_conversions: bool,
}

pub fn analyze_error_requirements(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    struct_level_error_type: &Option<syn::Type>,
) -> ErrorRequirements {
    let needs_try_from = requires_try_from(fields);
    let needs_default_error = requires_default_error_type(fields, struct_level_error_type);
    let needs_error_conversions = needs_try_from && needs_default_error && struct_level_error_type.is_none();

    ErrorRequirements {
        needs_try_from,
        needs_default_error,
        needs_error_conversions,
    }
}