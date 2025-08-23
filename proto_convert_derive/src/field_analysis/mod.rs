use crate::expect_analysis::ExpectMode;
use super::*;

mod proto_inspection;

#[derive(Clone)]
pub struct FieldProcessingContext<'a> {
    pub struct_name: &'a syn::Ident,
    pub field_name: &'a syn::Ident,
    pub field_type: &'a syn::Type,
    pub proto_field_ident: syn::Ident,
    pub proto_meta: attribute_parser::ProtoFieldMeta,
    pub expect_mode: ExpectMode,
    pub has_default: bool,
    pub default_fn: Option<String>,
    pub error_name: &'a syn::Ident,
    pub struct_level_error_type: &'a Option<syn::Type>,
    pub struct_level_error_fn: &'a Option<String>,
    pub proto_module: &'a str,
    pub proto_name: &'a str,
}

impl<'a> std::fmt::Debug for FieldProcessingContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldProcessingContext")
            .field("struct_name", &self.struct_name)
            .field("field_name", &self.field_name)
            .field("proto_field_ident", &self.proto_field_ident)
            .field("proto_meta", &self.proto_meta)
            .field("expect_mode", &self.expect_mode)
            .field("has_default", &self.has_default)
            .field("default_fn", &self.default_fn)
            .field("error_name", &self.error_name)
            .field("struct_level_error_fn", &self.struct_level_error_fn)
            .field("proto_module", &self.proto_module)
            .field("proto_name", &self.proto_name)
            .finish()
    }
}

impl<'a> FieldProcessingContext<'a> {
    pub fn new(
        struct_name: &'a syn::Ident,
        field: &'a syn::Field,
        error_name: &'a syn::Ident,
        struct_level_error_type: &'a Option<syn::Type>,
        struct_level_error_fn: &'a Option<String>,
        proto_module: &'a str,
        proto_name: &'a str,
    ) -> Self {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let proto_meta = attribute_parser::ProtoFieldMeta::from_field(field).unwrap_or_default();
        let expect_mode = ExpectMode::from_field_meta(field, &proto_meta);
        let has_default = proto_meta.default_fn.is_some();
        let default_fn = proto_meta.default_fn.clone();

        let proto_field_ident = if let Some(rename) = attribute_parser::get_proto_rename(field) {
            syn::Ident::new(&rename, proc_macro2::Span::call_site())
        } else {
            field_name.clone()
        };

        Self {
            struct_name,
            field_name,
            field_type,
            proto_field_ident,
            proto_meta,
            expect_mode,
            has_default,
            default_fn,
            error_name,
            struct_level_error_type,
            struct_level_error_fn,
            proto_module,
            proto_name,
        }
    }
}

pub use proto_inspection::detect_proto_field_optionality;

pub fn is_optional_proto_field_for_ctx(ctx: &FieldProcessingContext, field: &syn::Field) -> bool {
    // 1) check if user explicitly specified optionality
    if let Some(explicit) = ctx.proto_meta.optional {
        explicit
    } else if let Some(build_detected) = detect_proto_field_optionality(ctx) {
        // 2) try build-time metadata detection
        build_detected
    } else {
        // 3) fallback to original analysis
        is_optional_proto_field(ctx.struct_name, field, ctx.proto_name)
    }
}

fn is_optional_proto_field(name: &syn::Ident, field: &syn::Field, proto_name: &str) -> bool {
    let field_name = field.ident.as_ref().unwrap();

    if let Ok(proto_meta) = attribute_parser::ProtoFieldMeta::from_field(field) {
        if debug::should_output_debug(name, &field_name) {
            eprintln!("=== PROTO META DEBUG for {}.{} ===", proto_name, field_name);
            eprintln!("  proto_meta.optional: {:?}", proto_meta.optional);
        }

        if let Some(optional) = proto_meta.optional {
            if debug::should_output_debug(name, &field_name) {
                eprintln!("  RETURNING explicit optional = {optional}");
            }
            return optional;
        }
    }

    false
}
