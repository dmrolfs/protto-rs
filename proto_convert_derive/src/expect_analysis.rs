use super::*;

#[derive(Debug, Clone)]
pub enum ExpectMode {
    None,
    Error,
    Panic,
}

impl ExpectMode {
    pub fn from_field_meta(field: &Field, proto_meta: &attribute_parser::ProtoFieldMeta) -> ExpectMode {
        let field_name = field.ident.as_ref().unwrap();
        let expect_panic = has_expect_panic_syntax(field);

        let struct_name = syn::Ident::new("DEBUG", proc_macro2::Span::call_site());
        if debug::should_output_debug(&struct_name, field_name) {
            eprintln!("=== determine_expect_mode for {field_name} ===");
            eprintln!("  parse_expect_panic: {expect_panic}");
            eprintln!("  proto_meta.expect: {}", proto_meta.expect);
        }

        if expect_panic {
            ExpectMode::Panic
        } else if proto_meta.expect {
            ExpectMode::Error
        } else {
            ExpectMode::None
        }
    }
}

pub fn has_expect_panic_syntax(field: &Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens_str = meta_list.tokens.to_string();
                if tokens_str.replace(" ", "").contains("expect(panic)") {
                    return true;
                }
            }
        }
    }
    false
}
