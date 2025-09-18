use crate::analysis::attribute_parser;
use crate::analysis::expect_analysis::ExpectMode;
use quote::quote;

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

        let proto_field_ident = attribute_parser::get_proto_field_name(field)
            .map(|proto_name| syn::Ident::new(&proto_name, proc_macro2::Span::call_site()))
            .unwrap_or_else(|| field_name.clone());

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
            struct_level_error_fn,
            proto_module,
            proto_name,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CollectionType {
    Map,
    Vec,
    Set,
    Deque,
}

impl CollectionType {
    pub fn from_field_type(field_type: &syn::Type) -> Option<Self> {
        let type_str = quote!(#field_type).to_string();

        if type_str.contains("HashMap") || type_str.contains("BTreeMap") {
            Some(Self::Map)
        } else if type_str.contains("Vec") {
            Some(Self::Vec)
        } else if type_str.contains("HashSet") || type_str.contains("BTreeSet") {
            Some(Self::Set)
        } else if type_str.contains("VecDeque") {
            Some(Self::Deque)
        } else {
            None
        }
    }
}
