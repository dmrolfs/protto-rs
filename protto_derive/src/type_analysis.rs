use super::*;
use constants::PRIMITIVE_TYPES;

pub fn is_option_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(type_path) if type_path.path.segments.first().map(|s| s.ident == "Option").unwrap_or(false))
}

pub fn get_inner_type_from_option(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
        && type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option"
        && let syn::PathArguments::AngleBracketed(angle_bracketed) = &type_path.path.segments[0].arguments
        && let Some(syn::GenericArgument::Type(inner_type)) = angle_bracketed.args.first() {
            Some(inner_type.clone())
    } else {
        None
    }
}

pub fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
    && type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" {
            true
    } else {
        false
    }
}

pub fn get_inner_type_from_vec(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
    && type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec"
    && let syn::PathArguments::AngleBracketed(angle_bracketed) = &type_path.path.segments[0].arguments
    && let Some(syn::GenericArgument::Type(inner_type)) = angle_bracketed.args.first() {
        Some(inner_type.clone())
    } else {
        None
    }
}

pub fn is_primitive_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.len() == 1
            && PRIMITIVE_TYPES
                .iter()
                .any(|&p| type_path.path.segments[0].ident == p)
    } else {
        false
    }
}

/// Unified detection for any non-primitive, non-collection custom type
pub fn is_custom_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        // Skip primitives, collections, and proto types
        if is_primitive_type(ty) || is_vec_type(ty) || is_option_type(ty) {
            return false;
        }

        // Skip proto module types
        if type_path
            .path
            .segments
            .first()
            .map(|segment| segment.ident == "proto")
            .unwrap_or(false)
        {
            return false;
        }

        // Any remaining single-segment type is a custom type (struct, enum, newtype)
        // Let the Rust compiler and From trait implementations determine what works
        type_path.path.segments.len() == 1
    } else {
        false
    }
}

pub fn is_proto_type(ty: &Type, proto_module: &str) -> bool {
    if let Type::Path(type_path) = ty
    && let Some(segment) = type_path.path.segments.first() {
        segment.ident == proto_module
    } else {
        false
    }
}

pub fn is_enum_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
    && let Some(last_segment) = type_path.path.segments.last() {
        let type_name = last_segment.ident.to_string();
        registry::is_registered_enum_type(&type_name)
    } else {
        false
    }
}
