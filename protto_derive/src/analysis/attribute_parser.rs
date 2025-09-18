use crate::analysis::optionality::FieldOptionality;
use crate::constants;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Attribute, Expr, Field, Lit, Meta};

#[derive(Debug, Default, Clone)]
pub struct ProtoFieldMeta {
    pub expect: bool,
    pub error_fn: Option<String>,
    pub error_type: Option<String>,
    pub default_fn: Option<String>,
    pub optionality: Option<FieldOptionality>,
    pub from_proto_fn: Option<String>,
    pub to_proto_fn: Option<String>,
}

impl ProtoFieldMeta {
    pub fn from_field(field: &syn::Field) -> Result<Self, String> {
        let mut meta = ProtoFieldMeta::default();
        let field_name = field
            .ident
            .as_ref()
            .map(|i| i.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        for attr in &field.attrs {
            if attr.path().is_ident(constants::PROTTO_ATTRIBUTE)
                && let Meta::List(meta_list) = &attr.meta
            {
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
                                    if meta.optionality.is_some() {
                                        return Err(
                                            "Cannot specify both proto_optional and proto_required"
                                                .to_string(),
                                        );
                                    }
                                    meta.optionality = Some(FieldOptionality::Optional);
                                }
                                Meta::Path(path) if path.is_ident("proto_required") => {
                                    if meta.optionality.is_some() {
                                        return Err(
                                            "Cannot specify both proto_optional and proto_required"
                                                .to_string(),
                                        );
                                    }
                                    meta.optionality = Some(FieldOptionality::Required);
                                }

                                Meta::NameValue(nv) if nv.path.is_ident("error_type") => {
                                    if let Expr::Path(expr_path) = &nv.value {
                                        meta.error_type = Some(quote!(#expr_path).to_string());
                                    }
                                }
                                Meta::NameValue(nv) if nv.path.is_ident("error_fn") => {
                                    match parse_function_value(&nv.value, "error_fn", &field_name) {
                                        Ok(fn_name) => meta.error_fn = Some(fn_name),
                                        Err(err_msg) => return Err(err_msg),
                                    }
                                }

                                Meta::NameValue(nv) if nv.path.is_ident("default") => {
                                    if meta.default_fn.is_some() {
                                        return Err(format!(
                                            "Field '{}': Cannot specify both 'default' and 'default_fn'. \
                                                Use 'default = \"function_name\"' for custom default functions.",
                                            field_name
                                        ));
                                    }
                                    match &nv.value {
                                        Expr::Lit(expr_lit) => {
                                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                                let fn_name = lit_str.value();
                                                meta.default_fn = Some(fn_name);
                                            }
                                        }
                                        Expr::Path(expr_path) => {
                                            let fn_name = quote!(#expr_path).to_string();
                                            meta.default_fn = Some(fn_name);
                                        }
                                        _ => {
                                            return Err(format!(
                                                "Field '{}': default value must be a string literal or path. \
                                                    Examples: default = \"my_function\" or default = my_function",
                                                field_name
                                            ));
                                        }
                                    }
                                }
                                Meta::NameValue(nv) if nv.path.is_ident("default_fn") => {
                                    if meta.default_fn.is_some() {
                                        return Err(format!(
                                            "Field '{}': Cannot specify both 'default' and 'default_fn'. \
                                                Use 'default = \"function_name\"' instead.",
                                            field_name
                                        ));
                                    }
                                    match &nv.value {
                                        Expr::Lit(expr_lit) => {
                                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                                let fn_name = lit_str.value();
                                                meta.default_fn = Some(fn_name);
                                            }
                                        }
                                        Expr::Path(expr_path) => {
                                            let fn_name = quote!(#expr_path).to_string();
                                            meta.default_fn = Some(fn_name);
                                        }
                                        _ => {
                                            return Err(format!(
                                                "Field '{}': default_fn value must be a string literal or path. \
                                                    Examples: default_fn = \"my_function\" or default_fn = my_function",
                                                field_name
                                            ));
                                        }
                                    }
                                }
                                // Handle bare 'default' to use Default::default - add to separate field
                                Meta::Path(path) if path.is_ident("default") => {
                                    if meta.default_fn.is_some() {
                                        return Err(format!(
                                            "Field '{}': Cannot specify both 'default' and 'default_fn'. \
                                                Use 'default' for Default::default() or 'default_fn = \"function\"' for custom functions.",
                                            field_name
                                        ));
                                    }
                                    // Use a special marker to distinguish from custom default_fn
                                    meta.default_fn = Some(constants::USE_DEFAULT_IMPL.to_string());
                                }
                                Meta::Path(path) if path.is_ident("default_fn") => {
                                    return Err(format!(
                                        "Field '{}': 'default_fn' requires a value. \
                                            Use 'default_fn = \"function_name\"' or 'default = \"function_name\"'.",
                                        field_name
                                    ));
                                }

                                Meta::NameValue(nv) if nv.path.is_ident("from_proto_fn") => {
                                    match parse_function_value(
                                        &nv.value,
                                        "from_proto_fn",
                                        &field_name,
                                    ) {
                                        Ok(fn_name) => meta.from_proto_fn = Some(fn_name),
                                        Err(err_msg) => return Err(err_msg),
                                    }
                                }

                                Meta::NameValue(nv) if nv.path.is_ident("to_proto_fn") => {
                                    match parse_function_value(
                                        &nv.value,
                                        "to_proto_fn",
                                        &field_name,
                                    ) {
                                        Ok(fn_name) => meta.to_proto_fn = Some(fn_name),
                                        Err(err_msg) => return Err(err_msg),
                                    }
                                }

                                _ => {
                                    // ignore other attributes for now
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(format!(
                            "Failed to parse {} attribute: {e}",
                            constants::PROTTO_ATTRIBUTE
                        ));
                    }
                }
            }
        }

        Ok(meta)
    }

    /// Get the explicit proto optionality flag if present
    #[allow(unused)]
    pub fn get_proto_optionality(&self) -> Option<&FieldOptionality> {
        self.optionality.as_ref()
    }

    /// Check if this field is explicitly marked as proto optional
    #[allow(unused)]
    pub fn is_proto_optional(&self) -> bool {
        self.optionality.map(|o| o.is_optional()).unwrap_or(false)
    }

    /// Check if this field is explicitly marked as proto required
    #[allow(unused)]
    pub fn is_proto_required(&self) -> bool {
        self.optionality.map(|o| o.is_required()).unwrap_or(false)
    }

    /// Check if any explicit optionality annotation is present
    pub fn has_explicit_optionality(&self) -> bool {
        self.optionality.is_some()
    }

    /// Get the proto-to-rust conversion function if specified
    pub fn get_proto_to_rust_fn(&self) -> Option<&str> {
        self.from_proto_fn.as_deref()
    }

    /// Get the rust-to-proto conversion function if specified
    pub fn get_rust_to_proto_fn(&self) -> Option<&str> {
        self.to_proto_fn.as_deref()
    }

    /// Check if bidirectional custom conversion is specified
    #[allow(unused)]
    pub fn has_bidirectional_conversion(&self) -> bool {
        self.from_proto_fn.is_some() && self.to_proto_fn.is_some()
    }

    /// Check if any custom conversion function is specified
    #[allow(unused)]
    pub fn has_custom_conversion(&self) -> bool {
        self.from_proto_fn.is_some() || self.to_proto_fn.is_some()
    }
}

pub fn get_proto_struct_error_type(attrs: &[Attribute]) -> Option<syn::Type> {
    for attr in attrs {
        if attr.path().is_ident(constants::PROTTO_ATTRIBUTE)
            && let Meta::List(meta_list) = &attr.meta
        {
            let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                .parse2(meta_list.tokens.clone())
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to parse {} attribute: {}",
                        constants::PROTTO_ATTRIBUTE,
                        e
                    )
                });
            for meta in nested_metas {
                if let Meta::NameValue(meta_nv) = meta
                    && meta_nv.path.is_ident("error_type")
                {
                    if let Expr::Path(expr_path) = &meta_nv.value {
                        return Some(syn::Type::Path(syn::TypePath {
                            qself: None,
                            path: expr_path.path.clone(),
                        }));
                    }
                    panic!(
                        "error_type value must be a type path; e.g., #[{}(error_type = MyError)]",
                        constants::PROTTO_ATTRIBUTE
                    );
                }
            }
        }
    }
    None
}

pub fn get_struct_level_error_fn(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident(constants::PROTTO_ATTRIBUTE)
            && let Meta::List(meta_list) = &attr.meta
        {
            let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                .parse2(meta_list.tokens.clone())
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to parse {} attribute: {e}",
                        constants::PROTTO_ATTRIBUTE
                    )
                });
            for meta in nested_metas {
                if let Meta::NameValue(meta_nv) = meta
                    && meta_nv.path.is_ident("error_fn")
                {
                    if let Expr::Lit(expr_lit) = &meta_nv.value
                        && let Lit::Str(lit_str) = &expr_lit.lit
                    {
                        return Some(lit_str.value());
                    }
                    panic!("error_fn value must be a string literal");
                }
            }
        }
    }
    None
}

pub fn get_proto_module(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident(constants::PROTTO_ATTRIBUTE)
            && let Meta::List(meta_list) = &attr.meta
        {
            let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                .parse2(meta_list.tokens.clone())
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to parse {} attribute: {e}",
                        constants::PROTTO_ATTRIBUTE
                    )
                });
            for meta in nested_metas {
                if let Meta::NameValue(meta_nv) = meta
                    && meta_nv.path.is_ident("module")
                {
                    if let Expr::Lit(expr_lit) = meta_nv.value
                        && let Lit::Str(lit_str) = expr_lit.lit
                    {
                        return Some(lit_str.value());
                    }
                    panic!(
                        "module value must be a string literal, e.g., #[{}(module = \"path\")]",
                        constants::PROTTO_ATTRIBUTE
                    );
                }
            }
        }
    }
    None
}

pub fn get_proto_struct_name(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident(constants::PROTTO_ATTRIBUTE)
            && let Meta::List(meta_list) = &attr.meta
        {
            let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                .parse2(meta_list.tokens.clone())
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to parse {} attribute: {e}",
                        constants::PROTTO_ATTRIBUTE
                    )
                });
            for meta in nested_metas {
                if let Meta::NameValue(meta_nv) = meta
                    && meta_nv.path.is_ident("proto_name")
                {
                    if let Expr::Lit(expr_lit) = meta_nv.value
                        && let Lit::Str(lit_str) = expr_lit.lit
                    {
                        return Some(lit_str.value());
                    }
                    panic!(
                        "proto_name value must be a string literal, e.g., #[{}(proto_name = \"...\")]",
                        constants::PROTTO_ATTRIBUTE
                    );
                }
            }
        }
    }
    None
}

pub fn has_transparent_attr(field: &Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident(constants::PROTTO_ATTRIBUTE)
            && let Meta::List(meta_list) = &attr.meta
        {
            let tokens = &meta_list.tokens;
            let token_str = quote!(#tokens).to_string();
            if token_str.contains("transparent") {
                return true;
            }
        }
    }
    false
}

pub fn get_proto_field_name(field: &Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident(constants::PROTTO_ATTRIBUTE)
            && let Meta::List(meta_list) = &attr.meta
        {
            let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                .parse2(meta_list.tokens.clone())
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to parse {} attribute: {e}",
                        constants::PROTTO_ATTRIBUTE
                    )
                });
            for meta in nested_metas {
                if let Meta::NameValue(meta_nv) = meta
                    && meta_nv.path.is_ident("proto_name")
                {
                    if let Expr::Lit(expr_lit) = &meta_nv.value
                        && let Lit::Str(lit_str) = &expr_lit.lit
                    {
                        return Some(lit_str.value());
                    }
                    panic!(
                        "proto_name value must be a string literal, e.g., proto_name = \"field_name\""
                    );
                }
            }
        }
    }
    None
}

pub fn has_proto_ignore(field: &Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident(constants::PROTTO_ATTRIBUTE)
            && let Meta::List(meta_list) = &attr.meta
        {
            let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                .parse2(meta_list.tokens.clone())
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to parse {} attribute: {e}",
                        constants::PROTTO_ATTRIBUTE
                    )
                });
            for meta in nested_metas {
                if let Meta::Path(path) = meta
                    && path.is_ident("ignore")
                {
                    return true;
                }
            }
        }
    }
    false
}

fn parse_function_value(value: &Expr, attr_name: &str, field_name: &str) -> Result<String, String> {
    match value {
        Expr::Lit(expr_lit) => {
            if let Lit::Str(lit_str) = &expr_lit.lit {
                Ok(lit_str.value())
            } else {
                Err(format!(
                    "Field '{}': {} value must be a string literal or path. \
                    Examples: {} = \"my_function\" or {} = my_function",
                    field_name, attr_name, attr_name, attr_name
                ))
            }
        }
        Expr::Path(expr_path) => Ok(quote!(#expr_path).to_string()),
        _ => Err(format!(
            "Field '{}': {} value must be a string literal or path. \
                Examples: {} = \"my_function\" or {} = my_function",
            field_name, attr_name, attr_name, attr_name
        )),
    }
}
