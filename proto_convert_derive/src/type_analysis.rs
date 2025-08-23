use super::*;
use constants::PRIMITIVE_TYPES;

pub fn is_option_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(type_path) if type_path.path.segments.first().map(|s| s.ident == "Option").unwrap_or(false))
}

pub fn get_inner_type_from_option(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option" {
            if let syn::PathArguments::AngleBracketed(angle_bracketed) =
                &type_path.path.segments[0].arguments
            {
                if let Some(syn::GenericArgument::Type(inner_type)) = angle_bracketed.args.first() {
                    return Some(inner_type.clone());
                }
            }
        }
    }
    None
}

pub fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" {
            return true;
        }
    }
    false
}

pub fn get_inner_type_from_vec(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" {
            if let syn::PathArguments::AngleBracketed(angle_bracketed) =
                &type_path.path.segments[0].arguments
            {
                if let Some(syn::GenericArgument::Type(inner_type)) = angle_bracketed.args.first() {
                    return Some(inner_type.clone());
                }
            }
        }
    }
    None
}

pub fn is_primitive_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.len() == 1 &&
            PRIMITIVE_TYPES.iter().any(|&p| type_path.path.segments[0].ident == p)
    } else {
        false
    }
}

pub fn is_proto_type_with_module(ty: &Type, proto_module: &str) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.first() {
            return segment.ident == proto_module;
        }
    }
    false
}

pub fn is_enum_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        // Skip primitive types, collections, and proto types
        if is_primitive_type(ty) || is_vec_type(ty) || is_option_type(ty) {
            return false;
        }

        let is_proto_type = type_path.path.segments.first()
            .map(|segment| segment.ident == "proto")
            .unwrap_or(false);

        if is_proto_type {
            return false;
        }

        // Single-segment non-primitive types are likely enums or simple structs
        type_path.path.segments.len() == 1
    } else {
        false
    }
}

pub fn is_enum_type_with_explicit_attr(ty: &Type, field: &Field) -> bool {
    attribute_parser::has_proto_enum_attr(field) || is_enum_type(ty)
}