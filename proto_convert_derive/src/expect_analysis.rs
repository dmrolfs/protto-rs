use super::*;
use crate::debug::CallStackDebug;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ExpectMode {
    None,
    Error,
    Panic,
}

impl ExpectMode {
    pub fn from_field_meta(
        field: &Field,
        proto_meta: &attribute_parser::ProtoFieldMeta,
    ) -> ExpectMode {
        let _trace = CallStackDebug::new(
            "ExpectMode::from_field_meta",
            "",
            field
                .ident
                .as_ref()
                .map(|f| f.to_string())
                .unwrap_or_default(),
        );

        let expect_panic = has_expect_panic_syntax(field);
        _trace.checkpoint_data(
            "determine_expect_mode",
            &[
                ("expect_panic", &expect_panic.to_string()),
                ("proto_meta.expect", &proto_meta.expect.to_string()),
            ],
        );

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
