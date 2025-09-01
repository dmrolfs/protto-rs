use super::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ProtoOptionalityFlag {
    ProtoOptional,
    ProtoRequired,
}

#[derive(Debug, Default, Clone)]
pub struct ProtoFieldMeta {
    pub expect: bool,
    pub error_fn: Option<String>,
    pub error_type: Option<String>,
    pub default_fn: Option<String>,
    pub optionality_flag: Option<ProtoOptionalityFlag>,
}

impl ProtoFieldMeta {
    pub fn from_field(field: &syn::Field) -> Result<Self, String> {
        let mut meta = ProtoFieldMeta::default();

        for attr in &field.attrs {
            if attr.path().is_ident("proto") {
                if let Meta::List(meta_list) = &attr.meta {
                    let nested_metas: Result<Punctuated<Meta, Comma>, _> =
                        Punctuated::parse_terminated.parse2(meta_list.tokens.clone());

                    match nested_metas {
                        Ok(metas) => {
                            for nested_meta in metas {
                                match nested_meta {
                                    Meta::Path(path) if path.is_ident("expect") => {
                                        meta.expect = true;
                                    }
                                    Meta::List(list) if list.path.is_ident("expect") => {
                                        // handle `expect(panic)` syntax
                                        meta.expect = true;
                                    }
                                    Meta::Path(path) if path.is_ident("proto_optional") => {
                                        if meta.optionality_flag.is_some() {
                                            return Err("Cannot specify both proto_optional and proto_required".to_string());
                                        }
                                        meta.optionality_flag =
                                            Some(ProtoOptionalityFlag::ProtoOptional);
                                    }
                                    Meta::Path(path) if path.is_ident("proto_required") => {
                                        if meta.optionality_flag.is_some() {
                                            return Err("Cannot specify both proto_optional and proto_required".to_string());
                                        }
                                        meta.optionality_flag =
                                            Some(ProtoOptionalityFlag::ProtoRequired);
                                    }
                                    Meta::NameValue(nv) if nv.path.is_ident("error_type") => {
                                        if let Expr::Path(expr_path) = &nv.value {
                                            meta.error_type = Some(quote!(#expr_path).to_string());
                                        }
                                    }
                                    Meta::NameValue(nv) if nv.path.is_ident("error_fn") => {
                                        if let Expr::Lit(expr_lit) = &nv.value {
                                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                                meta.error_fn = Some(lit_str.value());
                                            }
                                        }
                                    }
                                    Meta::NameValue(nv)
                                        if nv.path.is_ident("default_fn")
                                            || nv.path.is_ident("default") =>
                                    {
                                        match &nv.value {
                                            Expr::Lit(expr_lit) => {
                                                if let Lit::Str(lit_str) = &expr_lit.lit {
                                                    meta.default_fn = Some(lit_str.value());
                                                }
                                            }
                                            Expr::Path(expr_path) => {
                                                meta.default_fn =
                                                    Some(quote!(#expr_path).to_string());
                                            }
                                            _ => {
                                                panic!(
                                                    "default_fn value must be a string literal or path; e.g., default_fn = \"function_path\" or default_fn = function_path"
                                                );
                                            }
                                        }
                                    }
                                    Meta::Path(path)
                                        if path.is_ident("default_fn")
                                            || path.is_ident("default") =>
                                    {
                                        meta.default_fn = Some("Default::default".to_string());
                                    }
                                    _ => {
                                        // ignore other attributes for now
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            return Err(format!("Failed to parse proto attribute: {e}"));
                        }
                    }
                }
            }
        }

        Ok(meta)
    }

    /// Get the explicit proto optionality flag if present
    pub fn get_proto_optionality(&self) -> Option<&ProtoOptionalityFlag> {
        self.optionality_flag.as_ref()
    }

    /// Check if this field is explicitly marked as proto optional
    pub fn is_proto_optional(&self) -> bool {
        matches!(
            self.optionality_flag,
            Some(ProtoOptionalityFlag::ProtoOptional)
        )
    }

    /// Check if this field is explicitly marked as proto required
    pub fn is_proto_required(&self) -> bool {
        matches!(
            self.optionality_flag,
            Some(ProtoOptionalityFlag::ProtoRequired)
        )
    }

    /// Check if any explicit optionality annotation is present
    pub fn has_explicit_optionality(&self) -> bool {
        self.optionality_flag.is_some()
    }
}

pub fn get_proto_struct_error_type(attrs: &[Attribute]) -> Option<syn::Type> {
    for attr in attrs {
        if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                for meta in nested_metas {
                    if let Meta::NameValue(meta_nv) = meta {
                        if meta_nv.path.is_ident("error_type") {
                            if let Expr::Path(expr_path) = &meta_nv.value {
                                return Some(syn::Type::Path(syn::TypePath {
                                    qself: None,
                                    path: expr_path.path.clone(),
                                }));
                            }
                            panic!(
                                "error_type value must be a type path; e.g., #[proto(error_type = MyError)]"
                            );
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn get_struct_level_error_fn(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE)
            && let Meta::List(meta_list) = &attr.meta
        {
            let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                .parse2(meta_list.tokens.clone())
                .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {e}"));
            for meta in nested_metas {
                if let Meta::NameValue(meta_nv) = meta {
                    if meta_nv.path.is_ident("error_fn") {
                        if let Expr::Lit(expr_lit) = &meta_nv.value {
                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                return Some(lit_str.value());
                            }
                        }
                        panic!("error_fn value must be a string literal");
                    }
                }
            }
        }
    }
    None
}

pub fn get_proto_module(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                for meta in nested_metas {
                    if let Meta::NameValue(meta_nv) = meta {
                        if meta_nv.path.is_ident("module") {
                            if let Expr::Lit(expr_lit) = meta_nv.value {
                                if let Lit::Str(lit_str) = expr_lit.lit {
                                    return Some(lit_str.value());
                                }
                            }
                            panic!(
                                "module value must be a string literal, e.g., #[proto(module = \"path\")]"
                            );
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn get_proto_struct_rename(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                for meta in nested_metas {
                    if let Meta::NameValue(meta_nv) = meta {
                        if meta_nv.path.is_ident("rename") {
                            if let Expr::Lit(expr_lit) = meta_nv.value {
                                if let Lit::Str(lit_str) = expr_lit.lit {
                                    return Some(lit_str.value());
                                }
                            }
                            panic!(
                                "rename value must be a string literal, e.g., #[proto(rename = \"...\")]"
                            );
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn has_transparent_attr(field: &Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = &meta_list.tokens;
                let token_str = quote!(#tokens).to_string();
                if token_str.contains("transparent") {
                    return true;
                }
            }
        }
    }
    false
}

pub fn get_proto_rename(field: &Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                for meta in nested_metas {
                    if let Meta::NameValue(meta_nv) = meta {
                        if meta_nv.path.is_ident("rename") {
                            if let Expr::Lit(expr_lit) = &meta_nv.value {
                                if let Lit::Str(lit_str) = &expr_lit.lit {
                                    return Some(lit_str.value());
                                }
                            }
                            panic!("rename value must be a string literal, e.g., rename = \"xyz\"");
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn get_proto_derive_from_with(field: &Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                for meta in nested_metas {
                    if let Meta::NameValue(meta_nv) = meta {
                        if meta_nv.path.is_ident("derive_from_with") {
                            if let Expr::Lit(expr_lit) = &meta_nv.value {
                                if let Lit::Str(lit_str) = &expr_lit.lit {
                                    return Some(lit_str.value());
                                }
                            }
                            panic!(
                                "derive_from_with value must be a string literal, e.g., derive_from_with = \"path::to::function\""
                            );
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn get_proto_derive_into_with(field: &Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                for meta in nested_metas {
                    if let Meta::NameValue(meta_nv) = meta {
                        if meta_nv.path.is_ident("derive_into_with") {
                            if let Expr::Lit(expr_lit) = &meta_nv.value {
                                if let Lit::Str(lit_str) = &expr_lit.lit {
                                    return Some(lit_str.value());
                                }
                            }
                            panic!(
                                "derive_into_with value must be a string literal, e.g., derive_into_with = \"path::to::function\""
                            );
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn has_proto_ignore(field: &Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                for meta in nested_metas {
                    if let Meta::Path(path) = meta {
                        if path.is_ident("ignore") {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

pub fn has_proto_enum_attr(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse meta list: {e}"));

                for meta in nested_metas {
                    if let Meta::Path(path) = meta {
                        if path.is_ident("enum") {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}
