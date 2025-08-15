use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::parse::Parser;
use syn::{self, Attribute, DeriveInput, Expr, Field, Lit, Meta, Type};
use syn::{punctuated::Punctuated, token::Comma};

fn output_debug(name: &syn::Ident, field_name: &syn::Ident) -> bool {
    false
    // || name.to_string() == "ComprehensiveEnumStruct"
    // || name.to_string() == "Status"
    // || name.to_string() == "MultipleErrorTypesStruct"
    // || name.to_string() == "ExpectCustomErrorStruct" // Enable debug for this struct
    // || name.to_string() == "MixedBehaviorTrack"
    // || name.to_string() == "HasOptionalWithCustomError"
    // || name.to_string() == "CombinationStruct"
    // || name.to_string() == "ComplexExpectStruct"
    // || name.to_string().contains("TrackWith")
}

fn debug_generated_code(name: &syn::Ident, field_name: &syn::Ident, code: &proc_macro2::TokenStream, context: &str) {
    if output_debug(name, field_name) {
        eprintln!("=== GENERATED CODE DEBUG: {} for {} ===", context, field_name);
        eprintln!("  {}", code);
        eprintln!("=== END GENERATED CODE ===");
    }
}

// #[derive(Debug, FromField, Default)]
// #[darling(attributes(proto), default)]
#[derive(Debug, Default)]
struct ProtoFieldMeta {
    // #[darling(rename = "expect")]
    expect: bool,

    // #[darling(skip)] // handled explicitly
    // expect_panic: Option<String>, // Will be "panic" if set

    // error: Option<String>,
    error_fn: Option<String>,
    error_type: Option<String>,

    // #[darling(default)]
    default_fn: Option<String>, // for #[proto(default = "function_name")]

    optional: Option<bool>,
}

impl ProtoFieldMeta {
    fn from_field(field: &syn::Field) -> Result<Self, String> {
        let mut meta = ProtoFieldMeta::default();

        for attr in &field.attrs {
            if attr.path().is_ident("proto") {
                if let Meta::List(meta_list) = &attr.meta {
                    let nested_metas: Result<Punctuated<Meta, Comma>, _> = Punctuated::parse_terminated
                        .parse2(meta_list.tokens.clone());

                    match nested_metas {
                        Ok(metas) => {
                            for nested_meta in metas {
                                match nested_meta {
                                    Meta::Path(path) if path.is_ident("expect") => {
                                        meta.expect = true;
                                    },
                                    Meta::List(list) if list.path.is_ident("expect") => {
                                        // handle `expect(panic)` syntax
                                        meta.expect = true;
                                    },
                                    Meta::NameValue(nv) if nv.path.is_ident("optional") => {
                                        if let Expr::Lit(expr_lit) = &nv.value {
                                            if let Lit::Bool(lit_bool) = &expr_lit.lit {
                                                meta.optional = Some(lit_bool.value);
                                            }
                                        }
                                    },
                                    Meta::NameValue(nv) if nv.path.is_ident("error_type") => {
                                        if let Expr::Path(expr_path) = &nv.value {
                                            meta.error_type = Some(quote!(#expr_path).to_string());
                                        }
                                    },
                                    Meta::NameValue(nv) if nv.path.is_ident("error_fn") => {
                                        if let Expr::Lit(expr_lit) = &nv.value {
                                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                                meta.error_fn = Some(lit_str.value());
                                            }
                                        }
                                    },
                                    Meta::NameValue(nv) if nv.path.is_ident("default_fn") || nv.path.is_ident("default") => {
                                        match &nv.value {
                                            Expr::Lit(expr_lit) => {
                                                if let Lit::Str(lit_str) = &expr_lit.lit {
                                                    meta.default_fn = Some(lit_str.value());
                                                }
                                            },
                                            Expr::Path(expr_path) => {
                                                meta.default_fn = Some(quote!(#expr_path).to_string());
                                            },
                                            _ => {
                                                panic!("default_fn value must be a string literal or path; e.g., default_fn = \"function_path\" or default_fn = function_path");
                                            },
                                        }
                                    },
                                    Meta::Path(path) if path.is_ident("default_fn") || path.is_ident("default") => {
                                        meta.default_fn = Some("Default::default".to_string());
                                    },
                                    _ => {
                                        // ignore other attributes for now
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            return Err(format!("Failed to parse proto attribute: {e}"));
                        },
                    }
                }
            }
        }

        Ok(meta)
    }
}

fn parse_expect_panic(field: &Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens_str = meta_list.tokens.to_string();
                if tokens_str.contains("expect(panic)") || tokens_str.contains("expect ( panic )") {
                    return true;
                }
            }
        }
    }
    false
}

fn has_proto_default(field: &Field) -> bool {
    if let Ok(proto_meta) = ProtoFieldMeta::from_field(field) {
        return proto_meta.default_fn.is_some();
    }

    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas = Punctuated::<Meta, Comma>::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {e}"));
                for meta in nested_metas {
                    match meta {
                        Meta::Path(path) if path.is_ident("default_fn") || path.is_ident("default") => return true,
                        Meta::NameValue(meta_nv) if meta_nv.path.is_ident("default_fn") || meta_nv.path.is_ident("default") => return true,
                        _ => {},
                    }
                }
            }
        }
    }
    false
}

fn get_proto_default_fn(field: &Field) -> Option<String> {
    if let Ok(proto_meta) = ProtoFieldMeta::from_field(field) {
        if let Some(default_val) = &proto_meta.default_fn {
            if !default_val.is_empty() {
                return Some(default_val.clone())
            }
        }
    }

    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas = Punctuated::<Meta, Comma>::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {e}"));
                for meta in nested_metas {
                    if let Meta::NameValue(meta_nv) = meta {
                        match &meta_nv.value {
                            Expr::Lit(expr_lit) => {
                                if let Lit::Str(lit_str) = &expr_lit.lit {
                                    return Some(lit_str.value());
                                }
                            },
                            Expr::Path(expr_path) => {
                                return Some(quote!(#expr_path).to_string());
                            },
                            _ => {
                                panic!("default_fn value must be a string literal or path; e.g., default_fn = \"function_path\" or default_fn = function_path");
                            },
                        }
                    }
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
enum ExpectMode {
    None,
    Error,
    Panic,
}

#[proc_macro_derive(ProtoConvert, attributes(proto))]
pub fn proto_convert_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let error_name = default_error_name(name);
    let proto_module = get_proto_module(&ast.attrs).unwrap_or_else(|| "proto".to_string());
    let proto_name = get_proto_struct_rename(&ast.attrs).unwrap_or_else(|| name.to_string());

    let struct_level_error_type = get_proto_struct_error_type(&ast.attrs);
    let struct_level_error_fn = get_struct_level_error_fn(&ast.attrs);
    let default_error_type = struct_level_error_type.clone().unwrap_or_else(|| syn::parse_str("String").unwrap());

    let proto_path =
        syn::parse_str::<syn::Path>(&format!("{}::{}", proto_module, proto_name)).unwrap();

    match &ast.data {
        syn::Data::Struct(data_struct) => {
            match &data_struct.fields {
                syn::Fields::Named(fields_named) => {
                    let fields = &fields_named.named;
                    let primitives = ["i32", "u32", "i64", "u64", "f32", "f64", "bool", "String"];

                    let needs_try_from = fields.iter().any(|field| {
                        if has_proto_ignore(field) {
                            false
                        } else {
                            let proto_meta = ProtoFieldMeta::from_field(field).unwrap_or_default();
                            let expect_mode = determine_expect_mode(field, &proto_meta);
                            matches!(expect_mode, ExpectMode::Error)
                        }
                    });

                    let needs_default_error = fields.iter().any(|field| {
                        if has_proto_ignore(field) { return false; }
                        let proto_meta = ProtoFieldMeta::from_field(field).unwrap_or_default();
                        if matches!(determine_expect_mode(field, &proto_meta), ExpectMode::Error) {
                            let effective_error_type = get_effective_error_type(&proto_meta, &struct_level_error_type);
                            effective_error_type.is_none()
                        } else {
                            false
                        }
                    });

                    // generate *ConversionError if needed
                    let conversion_error_def = if needs_try_from &&
                        needs_default_error &&
                        struct_level_error_type.is_none() {
                        generate_conversion_error(&name)
                    } else {
                        quote! {}
                    };

                    // let custom_errors: std::collections::HashSet<_> = fields.iter()
                    //     .filter_map(|field| {
                    //         if has_proto_ignore(field) { return None; }
                    //         let proto_meta = ProtoFieldMeta::from_field(field).unwrap_or_default();
                    //         if matches!(determine_expect_mode(field, &proto_meta), ExpectMode::Error) {
                    //             let effective_error_type = get_effective_error_type(&proto_meta, &struct_level_error_type);
                    //             if let Some(error_type) = effective_error_type {
                    //                 let struct_has_error_fn = struct_level_error_fn.is_some();
                    //                 if proto_meta.error_fn.is_none() && !struct_has_error_fn {
                    //                     let field_name = field.ident.as_ref().unwrap();
                    //                     panic!("Field '{}': When using a custom error_type, you must also specify either struct-level or field-level error_fn", field_name);
                    //                 }
                    //                 Some(quote!(#error_type).to_string())
                    //             } else if let Some(_error_fn) = &proto_meta.error_fn {
                    //                 panic!("Field-level error_fn requires struct-level error_type; e.g., #[proto(error_type = MyError)] on the struct");
                    //             } else {
                    //                 None
                    //             }
                    //         } else {
                    //             None
                    //         }
                    //     })
                    //     .collect();

                    let error_conversions = if needs_try_from &&
                        needs_default_error &&
                        struct_level_error_type.is_none() {

                        quote! {
                            impl From<String> for #error_name {
                                fn from(err: String) -> Self {
                                    Self::MissingField(err)
                                }
                            }
                        }
                    } else {
                        quote ! {}
                    };

                    let from_proto_fields = fields.iter().map(|field| {
                        let field_name = field.ident.as_ref().unwrap();

                        if has_proto_ignore(field) {
                            let default_fn = get_proto_default_fn(field);
                            if let Some(default_fn_name) = default_fn {
                                let default_fn_path: syn::Path = syn::parse_str(&default_fn_name)
                                    .expect("Failed to parse default_fn function path");
                                quote! { #field_name: #default_fn_path() }
                            } else {
                                quote! { #field_name: Default::default() }
                            }
                        } else {
                            let proto_meta = ProtoFieldMeta::from_field(field).unwrap_or_default();
                            let expect_mode = determine_expect_mode(field, &proto_meta);

                            let has_default = has_proto_default(field);
                            let default_fn = get_proto_default_fn(field);

                            // let effective_error_type = get_effective_error_type(&proto_meta, &struct_level_error_type);

                            let proto_field_ident = if let Some(rename) = get_proto_rename(field) {
                                syn::Ident::new(&rename, Span::call_site())
                            } else {
                                field_name.clone()
                            };
                            let field_type = &field.ty;
                            let is_transparent = has_transparent_attr(field);
                            let derive_from_with = get_proto_derive_from_with(field);

                            if output_debug(name, field_name) {
                                eprintln!("=== Processing {}.{} field ===", name, field_name);
                                eprintln!("  field_type: {}", quote!(#field_type));
                                eprintln!("  is_option_type: {}", is_option_type(field_type));
                                eprintln!("  is_transparent: {}", is_transparent);

                                if let syn::Type::Path(type_path) = field_type {
                                    eprintln!("  is_enum_type_with_explicit_attr: {}", is_enum_type_with_explicit_attr(field_type, field));
                                    let primitives = ["i32", "u32", "i64", "u64", "f32", "f64", "bool", "String"];
                                    let is_primitive = type_path.path.segments.len() == 1 &&
                                        primitives.iter().any(|&p| type_path.path.segments[0].ident == p);
                                    eprintln!("  is_primitive: {}", is_primitive);

                                    let is_proto_type = type_path.path.segments.first()
                                        .is_some_and(|segment| segment.ident == proto_module.as_str());
                                    eprintln!("  is_proto_type: {}", is_proto_type);
                                    eprintln!("  proto_module: {}", proto_module);
                                }

                                let expect_mode = determine_expect_mode(field, &proto_meta);
                                eprintln!("  expect_mode: {:?}", expect_mode);
                                eprintln!("  proto_is_optional: {}", is_optional_proto_field(name, field, &proto_name));
                            }

                            if let Some(from_with_path) = derive_from_with {
                                let from_with_path: syn::Path = syn::parse_str(&from_with_path).expect("Failed to parse derive_from_with path");
                                quote! {
                                    #field_name: #from_with_path(proto_struct.#proto_field_ident)
                                }
                            } else if is_transparent {
                                match expect_mode {
                                    ExpectMode::Panic => {
                                        quote! {
                                            #field_name: <#field_type>::from(
                                                proto_struct.#proto_field_ident
                                                    .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                                            )
                                        }
                                    },
                                    ExpectMode::Error => {
                                        generate_error_handling(
                                            field_name,
                                            &proto_field_ident,
                                            field_type,
                                            &proto_meta,
                                            &error_name,
                                            &struct_level_error_type,
                                            &struct_level_error_fn,
                                        )
                                    },
                                    ExpectMode::None => {
                                        if has_default {
                                            let default_expr = generate_default_value(field_type, default_fn.as_deref());
                                            quote! {
                                                #field_name: <#field_type>::from(
                                                    proto_struct.#proto_field_ident
                                                        .unwrap_or_else(|| #default_expr)
                                                )
                                            }
                                        } else {
                                            quote! {
                                                #field_name: <#field_type>::from(proto_struct.#proto_field_ident)
                                            }
                                        }
                                    },
                                }
                            } else if is_option_type(field_type) {
                                let inner_type = get_inner_type_from_option(field_type).unwrap();
                                let has_default = has_proto_default(field);
                                let default_fn = get_proto_default_fn(field);

                                match expect_mode {
                                    ExpectMode::Panic => {
                                        quote! {
                                            #field_name: Some(proto_struct.#proto_field_ident
                                                .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                                                .into())
                                        }
                                    },
                                    ExpectMode::Error => {
                                        generate_error_handling(
                                            field_name,
                                            &proto_field_ident,
                                            field_type,
                                            &proto_meta,
                                            &error_name,
                                            &struct_level_error_type,
                                            &struct_level_error_fn,
                                        )
                                    },
                                    ExpectMode::None => {
                                        if has_default {
                                            let default_expr = generate_default_value(field_type, default_fn.as_deref());
                                            quote! {
                                                #field_name: proto_struct.#proto_field_ident
                                                    .map(#inner_type::from)
                                                    .map(Some)
                                                    .unwrap_or_else(|| #default_expr)
                                            }
                                        } else if is_vec_type(&inner_type) {
                                            quote! {
                                                #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                                            }
                                        } else {
                                            quote! {
                                                #field_name: proto_struct.#proto_field_ident.map(Into::into)
                                            }
                                        }
                                    },
                                }
                            } else if is_vec_type(field_type) {
                                if has_default {
                                    let default_expr = generate_default_value(field_type, default_fn.as_deref());
                                    match expect_mode {
                                        ExpectMode::Panic => {
                                            quote! {
                                                #field_name: if proto_struct.#proto_field_ident.is_empty() {
                                                    #default_expr
                                                } else {
                                                    proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                                                }
                                            }
                                        },
                                        ExpectMode::Error => {
                                            generate_error_handling(
                                                field_name,
                                                &proto_field_ident,
                                                field_type,
                                                &proto_meta,
                                                &error_name,
                                                &struct_level_error_type,
                                                &struct_level_error_fn,
                                            )
                                        },
                                        ExpectMode::None => {
                                            quote! {
                                                #field_name: if proto_struct.#proto_field_ident.is_empty() {
                                                    #default_expr
                                                } else {
                                                    proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                                                }
                                            }
                                        },
                                    }
                                } else {
                                    if let Some(inner_type) = get_inner_type_from_vec(field_type) {
                                        if is_proto_type_with_module(&inner_type, &proto_module) {
                                            quote! {
                                                #field_name: proto_struct.#proto_field_ident
                                            }
                                        } else {
                                            quote! {
                                                #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                                            }
                                        }
                                    } else {
                                        quote! {
                                            #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                                        }
                                    }
                                }
                            } else if let syn::Type::Path(type_path) = field_type {
                                let is_primitive = type_path.path.segments.len() == 1 &&
                                    primitives.iter().any(|&p| type_path.path.segments[0].ident == p);
                                let is_proto_type = type_path.path.segments.first()
                                    .is_some_and(|segment| segment.ident == proto_module.as_str());
                                let has_default = has_proto_default(field);
                                let default_fn = get_proto_default_fn(field);

                                if is_enum_type_with_explicit_attr(field_type, field) {
                                    let proto_is_optional = is_optional_proto_field(name, field, &proto_name);
                                    if proto_is_optional {
                                        match expect_mode {
                                            ExpectMode::Panic => {
                                                quote! {
                                                    #field_name: proto_struct.#proto_field_ident
                                                        .map(|v| v.into())
                                                        .unwrap_or_else(|| panic!("Field {} is required", stringify!(#proto_field_ident)))
                                                    // #field_name: proto_struct.#proto_field_ident
                                                    //     .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                                                    //     .into()
                                                }
                                            },
                                            ExpectMode::Error => {
                                                generate_error_handling(
                                                    field_name,
                                                    &proto_field_ident,
                                                    field_type,
                                                    &proto_meta,
                                                    &error_name,
                                                    &struct_level_error_type,
                                                    &struct_level_error_fn,
                                                )
                                            },
                                            ExpectMode::None => {
                                                if has_default {
                                                    let default_expr = generate_default_value(field_type, default_fn.as_deref());
                                                    quote! {
                                                        #field_name: proto_struct.#proto_field_ident
                                                            .map(#field_type::from)
                                                            .unwrap_or_else(|| #default_expr)
                                                    }
                                                } else {
                                                    quote! {
                                                        #field_name: #field_type::from(
                                                            proto_struct.#proto_field_ident
                                                                .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                                                        )
                                                    }
                                                }
                                            },
                                        }
                                    } else {
                                        // direct conversion for non-optional enum fields
                                        quote! {
                                            #field_name: proto_struct.#proto_field_ident.into()
                                        }
                                    }
                                } else if is_primitive {
                                    let proto_is_optional = is_optional_proto_field(name, field, &proto_name);
                                    let rust_is_option = is_option_type(field_type);

                                    if output_debug(name, field_name) {
                                        eprintln!("=== PRIMITIVE FIELD DEBUG ===");
                                        eprintln!("  field_name: {}.{}", name, field_name);
                                        eprintln!("  proto_is_optional (calculated): {}", proto_is_optional);
                                        eprintln!("  rust_is_option: {}", rust_is_option);
                                        eprintln!("  has_default: {}", has_default);
                                        eprintln!("  expect_mode: {:?}", expect_mode);
                                        eprintln!("  proto_field_ident: {}", proto_field_ident);
                                        eprintln!("  default_fn: {:?}", default_fn);
                                        eprintln!("  Expected generated code: {}:proto_struct.{}.unwrap_or(...)", field_name, proto_field_ident);
                                    }

                                    let generated_code = match expect_mode {
                                        ExpectMode::Panic => {
                                            if rust_is_option {
                                                quote! {
                                                    #field_name: proto_struct.#proto_field_ident
                                                        .map(|v| Some(v))
                                                        .unwrap_or_else(|| panic!("Field {} is required", stringify!(#proto_field_ident)))
                                                }
                                            } else {
                                                quote! {
                                                    #field_name: proto_struct.#proto_field_ident
                                                        .unwrap_or_else(|| panic!("Field {} is required", stringify!(#proto_field_ident)))
                                                }
                                            }
                                        },
                                        ExpectMode::Error => {
                                            generate_error_handling(
                                                field_name,
                                                &proto_field_ident,
                                                field_type,
                                                &proto_meta,
                                                &error_name,
                                                &struct_level_error_type,
                                                &struct_level_error_fn,
                                            )
                                        },
                                        ExpectMode::None => {
                                            if has_default {
                                                let default_expr = generate_default_value(field_type, default_fn.as_deref());
                                                if rust_is_option {
                                                    quote! {
                                                        #field_name: if proto_struct.#proto_field_ident == Default::default() {
                                                            Some(#default_expr)
                                                        } else {
                                                            Some(proto_struct.#proto_field_ident)
                                                        }
                                                    }
                                                } else {
                                                    if proto_is_optional {
                                                        quote! {
                                                            #field_name: proto_struct.#proto_field_ident
                                                                .unwrap_or_else(|| #default_expr)
                                                        }
                                                    } else {
                                                        quote! {
                                                            //DMR: determine which better: pros/cons
                                                            #field_name: proto_struct.#proto_field_ident.into()
                                                            // #field_name: if proto_struct.#proto_field_ident.is_empty() {
                                                            //     #default_expr
                                                            // } else {
                                                            //     proto_struct.#proto_field_ident.into()
                                                            // }
                                                        }
                                                    }
                                                }
                                            } else {
                                                // No default handling
                                                if rust_is_option {
                                                    quote! {
                                                        #field_name: Some(proto_struct.#proto_field_ident)
                                                    }
                                                } else {
                                                    if proto_is_optional {
                                                        quote! {
                                                            #field_name: proto_struct.#proto_field_ident
                                                                .unwrap_or_default()
                                                                // .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                                                        }
                                                    } else {
                                                        quote! {
                                                            #field_name: proto_struct.#proto_field_ident
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                    };

                                    debug_generated_code(name, field_name, &generated_code, "primitive field from_proto");
                                    generated_code
                                } else if is_proto_type {
                                    // for proto types, always expect and unwrap since they're typically required
                                    // unless explicitly annotated otherwise
                                    match expect_mode {
                                        ExpectMode::Panic => {
                                            let proto_is_optional = is_optional_proto_field(name, field, &proto_name);
                                            if proto_is_optional {
                                                quote! {
                                                    #field_name: proto_struct.#proto_field_ident
                                                        .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                                                        .into()
                                                }
                                            } else {
                                                quote! {
                                                    #field_name: proto_struct.#proto_field_ident.into()
                                                }
                                            }
                                        },
                                        ExpectMode::Error => {
                                            generate_error_handling(
                                                field_name,
                                                &proto_field_ident,
                                                field_type,
                                                &proto_meta,
                                                &error_name,
                                                &struct_level_error_type,
                                                &struct_level_error_fn,
                                            )
                                        },
                                        ExpectMode::None => {
                                            if has_default {
                                                let default_expr = generate_default_value(field_type, default_fn.as_deref());
                                                quote! {
                                                    #field_name: proto_struct.#proto_field_ident
                                                        .unwrap_or_else(|| #default_expr)
                                                }
                                            } else {
                                                quote! {
                                                    #field_name: proto_struct.#proto_field_ident
                                                        .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                                                }
                                            }
                                        },
                                    }
                                } else {
                                    let proto_is_optional = is_optional_proto_field(name, field, &proto_name);

                                    // custom types - check if proto field is optional
                                    if proto_is_optional {
                                        // proto field is optional (Option<T>), Rust field is T
                                        match expect_mode {
                                            ExpectMode::Panic => {
                                                quote! {
                                                    #field_name: proto_struct.#proto_field_ident
                                                        .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                                                        .into()
                                                }
                                            },
                                            ExpectMode::Error => {
                                                generate_error_handling(
                                                    field_name,
                                                    &proto_field_ident,
                                                    field_type,
                                                    &proto_meta,
                                                    &error_name,
                                                    &struct_level_error_type,
                                                    &struct_level_error_fn,
                                                )
                                            },
                                            ExpectMode::None => {
                                                if has_default {
                                                    let default_expr = generate_default_value(field_type, default_fn.as_deref());
                                                    quote! {
                                                        #field_name: proto_struct.#proto_field_ident
                                                            .map(#field_type::from)
                                                            .unwrap_or_else(|| #default_expr)
                                                    }
                                                } else {
                                                    quote! {
                                                        #field_name: proto_struct.#proto_field_ident
                                                            .map(Into::into)
                                                            .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                                                    }
                                                }
                                            },
                                        }
                                    } else {
                                        // non-optional proto field - direct conversion
                                        quote! {
                                            #field_name: proto_struct.#proto_field_ident.into()
                                        }
                                    }
                                }
                            } else {
                                panic!("Only path types are supported for field '{}'", field_name);
                            }
                        }
                    });

                    let from_my_fields = fields.iter().filter(|field| !has_proto_ignore(field)).map(|field| {
                        let field_name = field.ident.as_ref().unwrap();
                        let proto_field_ident = if let Some(rename) = get_proto_rename(field) {
                            syn::Ident::new(&rename, Span::call_site())
                        } else {
                            field_name.clone()
                        };
                        let field_type = &field.ty;
                        let is_transparent = has_transparent_attr(field);
                        let derive_into_with = get_proto_derive_into_with(field);

                        if let Some(into_with_path) = derive_into_with {
                            let into_with_path: syn::Path = syn::parse_str(&into_with_path).expect("Failed to parse derive_into_with path");
                            quote! {
                                #proto_field_ident: #into_with_path(my_struct.#field_name)
                            }
                        } else if is_transparent {
                            let proto_is_optional = is_optional_proto_field(name, field, &proto_name);
                            if proto_is_optional {
                                quote! {
                                    #proto_field_ident: Some(my_struct.#field_name.into())
                                }
                            } else {
                                quote! {
                                    #proto_field_ident: my_struct.#field_name.into()
                                }
                            }
                        } else if is_option_type(field_type) {
                            let inner_type = get_inner_type_from_option(field_type).unwrap();
                            if is_vec_type(&inner_type) {
                                quote! {
                                    #proto_field_ident: my_struct.#field_name
                                        .map(|vec| vec.into_iter().map(Into::into).collect())
                                }
                            } else {
                                quote! {
                                    #proto_field_ident: my_struct.#field_name.map(Into::into)
                                }
                            }
                        } else if is_vec_type(field_type) {
                            if let Some(inner_type) = get_inner_type_from_vec(field_type) {
                                if is_proto_type_with_module(&inner_type, &proto_module) {
                                    quote! {
                                        #proto_field_ident: my_struct.#field_name
                                    }
                                } else {
                                    quote! {
                                        #proto_field_ident: my_struct.#field_name.into_iter().map(Into::into).collect()
                                    }
                                }
                            } else {
                                quote! {
                                    #proto_field_ident: my_struct.#field_name.into_iter().map(Into::into).collect()
                                }
                            }
                        } else if let syn::Type::Path(type_path) = field_type {
                            let is_primitive = type_path.path.segments.len() == 1
                                && primitives.iter().any(|&p| type_path.path.segments[0].ident == p);
                            let is_proto_type = type_path.path.segments.first()
                                .is_some_and(|segment| segment.ident == proto_module.as_str());

                            if is_enum_type_with_explicit_attr(field_type, field) {
                                let proto_is_optional = is_optional_proto_field(name, field, &proto_name);
                                if proto_is_optional {
                                    quote! { #proto_field_ident: Some(my_struct.#field_name.into()) }
                                } else {
                                    quote! { #proto_field_ident: my_struct.#field_name.into() }
                                }
                            } else if is_primitive {
                                let rust_is_option = is_option_type(field_type);
                                let proto_is_optional = is_optional_proto_field(name, field, &proto_name);
                                if output_debug(name, field_name) {
                                    eprintln!("=== FROM_MY_FIELDS PRIMITIVE ===");
                                    eprintln!("  proto_is_optional: {}", proto_is_optional);
                                }

                                match (rust_is_option, proto_is_optional) {
                                    (true, false) => quote! { #proto_field_ident: my_struct.#field_name.unwrap_or_default() },
                                    (false, true) => quote! { #proto_field_ident: Some(my_struct.#field_name) },
                                    (true, true) => quote! { #proto_field_ident: my_struct.#field_name },
                                    (false, false) => quote! { #proto_field_ident: my_struct.#field_name },
                                }
                            } else if is_proto_type {
                                    quote! { #proto_field_ident: Some(my_struct.#field_name) }
                            } else {
                                // check if proto field is optional before wrapping in Some()
                                if is_optional_proto_field(name, field, &proto_name) {
                                    quote! { #proto_field_ident: Some(my_struct.#field_name.into()) }
                                } else {
                                    quote! { #proto_field_ident: my_struct.#field_name.into() }
                                }
                            }
                        } else {
                            panic!("Only path types are supported for field '{}'", field_name);
                        }
                    });

                    // let field_custom_error_type = fields.iter()
                    //     .filter_map(|field| {
                    //         if has_proto_ignore(field) { return None; }
                    //         let proto_meta = ProtoFieldMeta::from_field(field).unwrap_or_default();
                    //         if matches!(determine_expect_mode(field, &proto_meta), ExpectMode::Error) {
                    //             proto_meta.error_type.as_ref().map(|et| {
                    //                 syn::parse_str::<syn::Type>(et)
                    //                     .expect("Failed to parse custom error type")
                    //             })
                    //         } else {
                    //             None
                    //         }
                    //     })
                    //     .next();

                    let actual_error_type = if needs_try_from {
                        struct_level_error_type.clone().unwrap_or_else(|| {
                            syn::Type::Path(syn::TypePath {
                                qself: None,
                                path: syn::Path::from(error_name.clone()),
                            })
                        })
                    } else {
                        default_error_type.clone()
                    };

                    let gen = if needs_try_from {
                        quote! {
                            #conversion_error_def
                            #error_conversions

                            impl TryFrom<#proto_path> for #name {
                                type Error = #actual_error_type;

                                fn try_from(proto_struct: #proto_path) -> Result<Self, Self::Error> {
                                    Ok(Self {
                                        #(#from_proto_fields),*
                                    })
                                }
                            }

                            impl From<#name> for #proto_path {
                                fn from(my_struct: #name) -> Self {
                                    Self {
                                        #(#from_my_fields),*
                                    }
                                }
                            }
                        }
                    } else {
                        quote! {
                            impl From<#proto_path> for #name {
                                fn from(proto_struct: #proto_path) -> Self {
                                    Self {
                                        #(#from_proto_fields),*
                                    }
                                }
                            }

                            impl From<#name> for #proto_path {
                                fn from(my_struct: #name) -> Self {
                                    Self {
                                        #(#from_my_fields),*
                                    }
                                }
                            }
                        }
                    };
                    gen.into()
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    if fields_unnamed.unnamed.len() != 1 {
                        panic!("ProtoConvert only supports tuple structs with exactly one field, found {}", fields_unnamed.unnamed.len());
                    }
                    let inner_type = &fields_unnamed.unnamed[0].ty;
                    let gen = quote! {
                        impl From<#inner_type> for #name {
                            fn from(value: #inner_type) -> Self {
                                #name(value)
                            }
                        }

                        impl From<#name> for #inner_type {
                            fn from(my: #name) -> Self {
                                my.0
                            }
                        }
                    };
                    gen.into()
                }
                syn::Fields::Unit => {
                    panic!("ProtoConvert does not support unit structs");
                }
            }
        }

        syn::Data::Enum(data_enum) => {
            let variants = &data_enum.variants;
            let enum_name_str = name.to_string();
            let enum_prefix = enum_name_str.to_uppercase();
            let proto_enum_path: syn::Path = syn::parse_str(&format!("{}::{}", proto_module, name))
                .expect("Failed to parse proto enum path");

            // let from_i32_arms = variants.iter().map(|variant| {
            //     let variant_ident = &variant.ident;
            //     let variant_str = variant_ident.to_string();
            //     let direct_candidate = variant_str.clone();
            //     let screaming_variant = to_screaming_snake_case(&variant_str);
            //     let prefixed_candidate = format!("{}_{}", enum_prefix, screaming_variant);
            //     let direct_candidate_lit = syn::LitStr::new(&direct_candidate, Span::call_site());
            //     let prefixed_candidate_lit = syn::LitStr::new(&prefixed_candidate, Span::call_site());
            //     quote! {
            //         candidate if candidate == #direct_candidate_lit || candidate == #prefixed_candidate_lit => #name::#variant_ident,
            //     }
            // });

            let from_proto_enum_arms: Vec<_> = variants.iter().map(|variant| {
                let variant_ident = &variant.ident;
                let variant_str = variant_ident.to_string();
                let screaming_variant = to_screaming_snake_case(&variant_str);
                let prefixed_candidate = format!("{}_{}", enum_prefix, screaming_variant);

                quote! {
                    candidate if candidate == #variant_str || candidate == #prefixed_candidate => #name::#variant_ident,
                }
            })
                .collect();

            let from_proto_arms: Vec<_> = variants.iter().map(|variant| {
                let variant_ident = &variant.ident;
                let variant_str = variant_ident.to_string();
                let screaming_variant = to_screaming_snake_case(&variant_str);
                let prefixed_candidate = format!("{}_{}", enum_prefix, screaming_variant);
                let prefixed_candidate_lit = syn::LitStr::new(&prefixed_candidate, Span::call_site());
                quote! {
                    #name::#variant_ident => <#proto_enum_path>::from_str_name(#prefixed_candidate_lit)
                        .unwrap_or_else(|| panic!("No matching proto variant for {rust_enum:?}")),
                }
            })
                .collect();

            let gen = quote! {
                impl From<i32> for #name {
                    fn from(value: i32) -> Self {
                        eprintln!("DEBUG: Converting i32 {} to {}", value, stringify!(#name));
                        let proto_val = <#proto_enum_path>::from_i32(value)
                            .unwrap_or_else(|| panic!("Unknown enum value: {}", value));
                        let proto_str = proto_val.as_str_name();
                        match proto_str {
                            #(#from_proto_enum_arms)*
                            _ => panic!("No matching Rust variant for proto enum string: {}", proto_str),
                        }
                    }
                }

                impl From<#name> for i32 {
                    fn from(rust_enum: #name) -> Self {
                        let proto: #proto_enum_path = rust_enum.into();
                        proto as i32
                    }
                }

                impl From<#name> for #proto_enum_path {
                    fn from(rust_enum: #name) -> Self {
                        match rust_enum {
                            #(#from_proto_arms)*
                        }
                    }
                }

                impl From<#proto_enum_path> for #name {
                    fn from(proto_enum: #proto_enum_path) -> Self {
                        let proto_str = proto_enum.as_str_name();
                        match proto_str {
                            #(#from_proto_enum_arms)*
                            _ => panic!("No matching Rust variant for proto enum string: {proto_str}"),
                        }
                    }
                }
            };
            gen.into()
        }
        _ => panic!("ProtoConvert only supports structs and enums, not unions"),
    }
}

fn default_error_name(struct_name: &syn::Ident) -> Ident {
    syn::Ident::new(&format!("{struct_name}ConversionError"), struct_name.span())
}

// add *ConversionError type definition for when TryFrom is needed
fn generate_conversion_error(struct_name: &syn::Ident) -> proc_macro2::TokenStream {
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

// fn generate_error_conversion(error_name: &Ident, custom_error: &str) -> proc_macro2::TokenStream {
//     let custom_err_path: syn::Path = syn::parse_str(custom_error).expect("Failed to parse custom error path");
//
//     quote! {
//         impl From<#custom_err_path> for #error_name {
//             fn from(err: #custom_err_path) -> Self {
//                 Self::MissingField(err.to_string())
//             }
//         }
//     }
// }

fn generate_error_handling(
    field_name: &syn::Ident,
    proto_field_ident: &syn::Ident,
    field_type: &syn::Type,
    proto_meta: &ProtoFieldMeta,
    error_name: &Ident,
    _struct_level_error_type: &Option<syn::Type>,
    struct_level_error_fn: &Option<String>,
) -> proc_macro2::TokenStream {
    let is_rust_optional = is_option_type(field_type);

    let error_fn_to_use = proto_meta.error_fn.as_ref().or(struct_level_error_fn.as_ref());

    if let Some(error_fn) = error_fn_to_use {
        let error_fn_path: syn::Path = syn::parse_str(error_fn)
            .expect("Failed to parse error function path");

        if is_rust_optional {
            quote! {
                #field_name: Some(proto_struct.#proto_field_ident
                    .ok_or_else(|| #error_fn_path(stringify!(#proto_field_ident)))?
                    .into())
            }
        } else {
            quote! {
                #field_name: proto_struct.#proto_field_ident
                    .ok_or_else(|| #error_fn_path(stringify!(#proto_field_ident)))?
                    .into()
            }
        }
    } else {
        let error_expr = quote! { #error_name::MissingField(stringify!(#proto_field_ident).to_string()) };

        if is_rust_optional {
            quote! {
                #field_name: Some(proto_struct.#proto_field_ident
                    .ok_or_else(|| #error_expr)?
                    .into())
            }
        } else {
            quote! {
                #field_name: proto_struct.#proto_field_ident
                    .ok_or_else(|| #error_expr)?
                    .into()
            }
        }
    }
}

// fn is_string_error_type(error_type: &syn::Type) -> bool {
//     if let syn::Type::Path(type_path) = error_type {
//         if type_path.path.segments.len() == 1 {
//             return type_path.path.segments[0].ident == "String";
//         }
//     }
//     false
// }

fn generate_default_value(field_type: &syn::Type, default_fn: Option<&str>) -> proc_macro2::TokenStream {
    if let Some(default_fn_name) = default_fn {
        let default_fn_path: syn::Path = syn::parse_str(&default_fn_name)
            .expect("Failed to parse default_fn path");
        quote! { #default_fn_path() }
    } else {
        quote! { <#field_type as Default>::default() }
    }
}

// fn is_defaultable_type(ty: &Type) -> bool {
//     if let Type::Path(type_path) = ty {
//         if type_path.path.segments.len() == 1 {
//             let type_name = type_path.path.segments[0].ident.to_string();
//             matches!(type_name.as_str(),
//                 "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
//                 "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
//                 "f32" | "f64" | "bool" | "String" |
//                 "Vec" | "HashMap" | "BTreeMap" | "HashSet" | "BTreeSet"
//             )
//         } else {
//             false
//         }
//     } else {
//         false
//     }
// }

fn get_proto_struct_error_type(attrs: &[Attribute]) -> Option<syn::Type> {
    for attr in attrs {
        if attr.path().is_ident("proto") {
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
                            panic!("error_type value must be a type path; e.g., #[proto(error_type = MyError)]");
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_effective_error_type(proto_meta: &ProtoFieldMeta, struct_level_error_type: &Option<syn::Type>) -> Option<syn::Type> {
    if let Some(field_error_type) = &proto_meta.error_type {
        return Some(syn::parse_str(field_error_type)
            .expect("Failed to parse field-level error_type"));
    }

    struct_level_error_type.clone()
}

fn get_struct_level_error_fn(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("proto") {
            if let Meta::List(meta_list) = &attr.meta {
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
    }
    None
}

fn is_optional_proto_field(name: &syn::Ident, field: &syn::Field, proto_name: &str) -> bool {
    let field_name = field.ident.as_ref().unwrap();
    if output_debug(name, &field_name) {
        eprintln!("=== DEBUG is_optional_proto_field for {} ===", field_name);
        eprintln!("  has_proto_default: {}", has_proto_default(field));
        eprintln!("  proto field type from proto definition should be: Option<T>");
        eprintln!("=== PROTO NAME DEBUG: {} for field {} ===", proto_name, field_name);

        for (i, attr) in field.attrs.iter().enumerate() {
            eprintln!("  attr[{i}): {attr:?}");
        }
    }

    let proto_meta_result = ProtoFieldMeta::from_field(field);
    if let Ok(proto_meta) = &proto_meta_result {
        if output_debug(name, &field_name) {
            eprintln!("=== PROTO META DEBUG for {} ===", field_name);
            eprintln!("  proto_meta.optional: {:?}", proto_meta.optional);
            eprintln!("  proto_meta.default_fn: {:?}", proto_meta.default_fn);
            eprintln!("  proto_meta.expect: {:?}", proto_meta.expect);
        }

        if let Some(optional) = proto_meta.optional {
            if output_debug(name, &field_name) {
                eprintln!("  RETURNING explicit optional = {optional}");
            }
            return optional;
        }
    } else if output_debug(name, &field_name) {
        eprintln!("  FAILED to parse ProtoFieldMeta from field!");
    }

    let field_type = &field.ty;

    if is_option_type(field_type) {
        return true;
    }

    if is_vec_type(field_type) {
        return false;
    }

    if has_proto_default(field) {
        if let Ok(proto_meta) = &proto_meta_result {
            let expect_mode = determine_expect_mode(field, &proto_meta);
            if !matches!(expect_mode, ExpectMode::None) {
                return false;
            }

            // if it has a default (either bare "default_fn" or "default_fn = <function>"),
            // assume the proto field is optional since that's the typical use case
            return true;
        }

        // fallback: if has_proto_default but can't parse meta, assume optional
        return true;
    }

    let has_expect = parse_expect_panic(field) ||
        proto_meta_result.as_ref().map(|m| m.expect).unwrap_or(false);

    if output_debug(name, &field_name) {
        eprintln!("  has_expect: {has_expect}, returning: {has_expect}");
    }

    has_expect
}

// Helper to get proto module from current context (struct-level attributes)
// fn get_proto_module_from_context() -> Option<String> {
//     // This would need to be passed down from the main macro context
//     // For now, return None and fall back to default
//     None
// }

// helper function to detect enum types
fn is_enum_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        let primitives = ["i32", "u32", "i64", "u64", "f32", "f64", "bool", "String"];
        let is_primitive = type_path.path.segments.len() == 1 &&
            primitives.iter().any(|&p| type_path.path.segments[0].ident == p);

        if is_primitive || is_vec_type(ty) || is_option_type(ty) {
            return false;
        }

        let is_proto_type = type_path.path.segments.first()
            .map(|segment| segment.ident == "proto")
            .unwrap_or(false);

        if is_proto_type {
            return false;
        }

        if type_path.path.segments.len() == 1 {
            // We could check if the type has certain derive attributes
            // but that requires more context. For now, assume single-segment,
            // non-primitive, non-proto types are likely enums or simple structs.
            // This is imperfect but better than hardcoding specific names.
            true
        } else {
            false
        }
    } else {
        false
    }
}

fn is_enum_type_with_explicit_attr(ty: &Type, field: &Field) -> bool {
    has_proto_enum_attr(field) || is_enum_type(ty)
}

fn has_proto_enum_attr(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
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

// // helper function to get the proto module for a specific field
// // this checks field-level module override first, then falls back to struct-level
// fn get_proto_module_for_field(field: &syn::Field) -> Option<String> {
//     // first check if the field has its own module specification
//     for attr in &field.attrs {
//         if attr.path().is_ident("proto") {
//             if let Meta::List(meta_list) = &attr.meta {
//                 let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
//                     .parse2(meta_list.tokens.clone())
//                     .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {e}"));
//
//                 for meta in nested_metas {
//                     if let Meta::NameValue(meta_nv) = meta {
//                         if meta_nv.path.is_ident("module") {
//                             if let Expr::Lit(expr_lit) = &meta_nv.value {
//                                 if let Lit::Str(lit_str) = &expr_lit.lit {
//                                     return Some(lit_str.value());
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }
//
//     None
// }

fn determine_expect_mode(field: &Field, proto_meta: &ProtoFieldMeta) -> ExpectMode {
    let field_name = field.ident.as_ref().unwrap();
    let expect_panic = parse_expect_panic(field);

    let struct_name = syn::Ident::new("DEBUG", proc_macro2::Span::call_site());
    if output_debug(&struct_name, field_name) {
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

// fn get_proto_error_type(attrs: &[Attribute]) -> Option<syn::Type> {
//     for attr in attrs {
//         if attr.path().is_ident("proto") {
//             if let Meta::List(meta_list) = &attr.meta {
//                 let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
//                     .parse2(meta_list.tokens.clone())
//                     .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
//                 for meta in nested_metas {
//                     if let Meta::NameValue(meta_nv) = meta {
//                         if meta_nv.path.is_ident("error_type") {
//                             if let Expr::Path(expr_path) = &meta_nv.value {
//                                 return Some(syn::Type::Path(syn::TypePath {
//                                     qself: None,
//                                     path: expr_path.path.clone(),
//                                 }));
//                             }
//                             panic!("error_type value must be a type path, e.g., #[proto(error_type = MyError)]");
//                         }
//                     }
//                 }
//             }
//         }
//     }
//     None
// }

fn to_screaming_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i != 0 {
            result.push('_');
        }
        result.push(c.to_ascii_uppercase());
    }
    result
}

fn is_option_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(type_path) if type_path.path.segments.first().map(|s| s.ident == "Option").unwrap_or(false))
    // if let Type::Path(type_path) = ty {
    //     if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option" {
    //         return true;
    //     }
    // }
    // false
}

fn get_inner_type_from_option(ty: &Type) -> Option<Type> {
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

fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" {
            return true;
        }
    }
    false
}

fn get_inner_type_from_vec(ty: &Type) -> Option<Type> {
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

fn is_proto_type_with_module(ty: &Type, proto_module: &str) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.first() {
            return segment.ident == proto_module;
        }
    }
    false
}

fn get_proto_module(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("proto") {
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
                            panic!("module value must be a string literal, e.g., #[proto(module = \"path\")]");
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_proto_struct_rename(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("proto") {
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
                            panic!("rename value must be a string literal, e.g., #[proto(rename = \"...\")]");
                        }
                    }
                }
            }
        }
    }
    None
}

fn has_transparent_attr(field: &Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
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

fn get_proto_rename(field: &Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
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

fn get_proto_derive_from_with(field: &Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
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
                            panic!("derive_from_with value must be a string literal, e.g., derive_from_with = \"path::to::function\"");
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_proto_derive_into_with(field: &Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
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
                            panic!("derive_into_with value must be a string literal, e.g., derive_into_with = \"path::to::function\"");
                        }
                    }
                }
            }
        }
    }
    None
}

fn has_proto_ignore(field: &Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
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
