use super::*;
use field_analysis::FieldProcessingContext;
use crate::expect_analysis::ExpectMode;

pub fn generate_from_proto_field(field: &syn::Field, ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
    if debug::should_output_debug(ctx.struct_name, ctx.field_name) {
        debug::debug_field_analysis(ctx.struct_name, ctx.field_name, "GENERATE_FROM_PROTO_FIELD DEBUG", &[
            ("field_type", quote!(#(ctx.field_type)).to_string()),
            ("has_proto_ignore", attribute_parser::has_proto_ignore(field).to_string()),
            ("has_transparent_attr", attribute_parser::has_transparent_attr(field).to_string()),
            ("is_option_type", type_analysis::is_option_type(ctx.field_type).to_string()),
            ("is_vec_type", type_analysis::is_vec_type(ctx.field_type).to_string()),
        ]);
    }

    if attribute_parser::has_proto_ignore(field) {
        return generate_ignored_field(ctx);
    }

    let derive_from_with = attribute_parser::get_proto_derive_from_with(field);
    if let Some(from_with_path) = derive_from_with {
        return generate_derive_from_with_field(ctx, &from_with_path)
    }

    if attribute_parser::has_transparent_attr(field) {
        return generate_transparent_field(ctx);
    }

    if type_analysis::is_option_type(ctx.field_type) {
        return generate_option_field(ctx);
    }

    if type_analysis::is_vec_type(ctx.field_type) {
        return generate_vec_field(ctx);
    }

    if let syn::Type::Path(_) = ctx.field_type {
        return generate_path_type_field(ctx, field);
    }

    panic!("Only path types are supported for field '{}'", ctx.field_name);
}

pub fn generate_from_my_field(field: &syn::Field, ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
    if attribute_parser::has_proto_ignore(field) {
        // Ignored fields are not included in proto struct
        return quote!{};
    }

    let derive_into_with = attribute_parser::get_proto_derive_into_with(field);
    if let Some(into_with_path) = derive_into_with {
        return generate_derive_into_with_field(ctx, &into_with_path);
    }

    if attribute_parser::has_transparent_attr(field) {
        return generate_transparent_from_my_field(ctx, field);
    }

    if type_analysis::is_option_type(ctx.field_type) {
        return generate_option_from_my_field(ctx);
    }

    if type_analysis::is_vec_type(ctx.field_type) {
        return generate_vec_from_my_field(ctx);
    }

    if let syn::Type::Path(_) = ctx.field_type {
        return generate_path_type_from_my_field(ctx, field);
    }

    panic!("Only path types are supported for field '{}'", ctx.field_name);
}

fn generate_ignored_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    if let Some(default_fn_name) = &ctx.default_fn {
        let default_fn_path: syn::Path = syn::parse_str(&default_fn_name)
            .expect("Failed to parse default_fn function path");
        quote! { #field_name: #default_fn_path() }
    } else {
        quote! { #field_name: Default::default() }
    }
}

fn generate_derive_from_with_field(ctx: &FieldProcessingContext, from_with_path: &str) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;
    let from_with_path: syn::Path = syn::parse_str(&from_with_path).expect("Failed to parse derive_from_with path");
    quote! {
        #field_name: #from_with_path(proto_struct.#proto_field_ident)
    }
}

fn generate_transparent_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;
    let field_type = ctx.field_type;

    match ctx.expect_mode {
        ExpectMode::Panic => {
            quote! {
                #field_name: <#field_type>::from(
                    proto_struct.#proto_field_ident
                        .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                )
            }
        },
        ExpectMode::Error => {
            error_handler::generate_error_handling(
                field_name,
                &proto_field_ident,
                field_type,
                &ctx.proto_meta,
                ctx.error_name,
                ctx.struct_level_error_type,
                ctx.struct_level_error_fn,
            )
        },
        ExpectMode::None => {
            if ctx.has_default {
                let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
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
}

fn generate_option_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;
    let field_type = ctx.field_type;
    let inner_type = type_analysis::get_inner_type_from_option(field_type).unwrap();

    match ctx.expect_mode {
        ExpectMode::Panic => {
            quote! {
                #field_name: Some(proto_struct.#proto_field_ident
                    .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                    .into())
            }
        },
        ExpectMode::Error => {
            error_handler::generate_error_handling(
                field_name,
                &proto_field_ident,
                field_type,
                &ctx.proto_meta,
                ctx.error_name,
                ctx.struct_level_error_type,
                ctx.struct_level_error_fn,
            )
        },
        ExpectMode::None => {
            if ctx.has_default {
                let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
                quote! {
                    #field_name: proto_struct.#proto_field_ident
                        .map(#inner_type::from)
                        .map(Some)
                        .unwrap_or_else(|| #default_expr)
                }
            } else if type_analysis::is_vec_type(&inner_type) {
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
}

fn generate_vec_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;
    let field_type = ctx.field_type;

    if ctx.has_default {
        let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
        match ctx.expect_mode {
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
                error_handler::generate_error_handling(
                    field_name,
                    &proto_field_ident,
                    field_type,
                    &ctx.proto_meta,
                    ctx.error_name,
                    ctx.struct_level_error_type,
                    ctx.struct_level_error_fn,
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
        if let Some(inner_type) = type_analysis::get_inner_type_from_vec(field_type) {
            if type_analysis::is_proto_type_with_module(&inner_type, ctx.proto_module) {
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
}

fn generate_path_type_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let field_type = ctx.field_type;

    if let syn::Type::Path(type_path) = field_type {
        let is_primitive = type_analysis::is_primitive_type(field_type);
        let is_proto_type = type_path.path.segments.first()
            .is_some_and(|segment| segment.ident == ctx.proto_module);
        let is_enum = type_analysis::is_enum_type_with_explicit_attr(field_type, field);

        if debug::should_output_debug(ctx.struct_name, ctx.field_name) {
            debug::debug_field_analysis(ctx.struct_name, ctx.field_name, "PATH TYPE FIELD DEBUG", &[
                ("is_primitive", is_primitive.to_string()),
                ("is_proto_type", is_proto_type.to_string()),
                ("is_enum", is_enum.to_string()),
                ("proto_module", ctx.proto_module.to_string()),
                ("field_type", quote!(#field_type).to_string()),
            ]);
        }

        if is_enum {
            return generate_enum_field(ctx, field);
        } else if is_primitive {
            return generate_primitive_field(ctx, field);
        } else if is_proto_type {
            return generate_proto_type_field(ctx, field);
        } else {
            return generate_custom_type_field(ctx, field);
        }
    }

    panic!("Only path types are supported for field '{}'", field_name);
}

fn generate_enum_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;
    let field_type = ctx.field_type;

    let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

    if debug::should_output_debug(ctx.struct_name, field_name) {
        debug::debug_field_analysis(ctx.struct_name, field_name, "ENUM FIELD DEBUG", &[
            ("proto_is_optional (calculated)", proto_is_optional.to_string()),
            ("expect_mode", format!("{:?}", ctx.expect_mode)),
            ("has_default", ctx.has_default.to_string()),
            ("proto_field_ident", proto_field_ident.to_string()),
            ("field_type", quote!(#field_type).to_string()),
        ]);
    }

    let generated_code = if proto_is_optional {
        match ctx.expect_mode {
            ExpectMode::Panic => {
                quote! {
                    #field_name: proto_struct.#proto_field_ident
                        .map(|v| v.into())
                        .unwrap_or_else(|| panic!("Field {} is required", stringify!(#proto_field_ident)))
                }
            },
            ExpectMode::Error => {
                error_handler::generate_error_handling(
                    field_name,
                    &proto_field_ident,
                    field_type,
                    &ctx.proto_meta,
                    ctx.error_name,
                    ctx.struct_level_error_type,
                    ctx.struct_level_error_fn,
                )
            },
            ExpectMode::None => {
                if ctx.has_default {
                    let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
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
            // #field_name: proto_struct.#proto_field_ident.into()
            #field_name: #field_type::from(proto_struct.#proto_field_ident)
        }
    };

    debug::debug_generated_code(ctx.struct_name, field_name, &generated_code, "enum field from_proto");
    generated_code
}

fn generate_primitive_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;
    let field_type = ctx.field_type;

    let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

    let rust_is_option = type_analysis::is_option_type(field_type);

    if debug::should_output_debug(ctx.struct_name, field_name) {
        debug::debug_field_analysis(ctx.struct_name, field_name, "PRIMITIVE FIELD DEBUG", &[
            ("proto_is_optional (calculated)", proto_is_optional.to_string()),
            ("rust_is_option", rust_is_option.to_string()),
            ("has_default", ctx.has_default.to_string()),
            ("expect_mode", format!("{:?}", ctx.expect_mode)),
            ("proto_field_ident", proto_field_ident.to_string()),
            ("default_fn", format!("{:?}", ctx.default_fn)),
            ("Expected generated code", format!("{}:proto_struct.{}.unwrap_or(...)", field_name, proto_field_ident)),
        ]);
    }

    let generated_code = match ctx.expect_mode {
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
            error_handler::generate_error_handling(
                field_name,
                &proto_field_ident,
                field_type,
                &ctx.proto_meta,
                ctx.error_name,
                ctx.struct_level_error_type,
                ctx.struct_level_error_fn,
            )
        },
        ExpectMode::None => {
            if ctx.has_default {
                let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
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
                            //todo: determine which better: pros/cons
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

    debug::debug_generated_code(ctx.struct_name, field_name, &generated_code, "primitive field from_proto");
    generated_code
}

fn generate_proto_type_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;

    match ctx.expect_mode {
        ExpectMode::Panic => {
            let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

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
            error_handler::generate_error_handling(
                field_name,
                &proto_field_ident,
                ctx.field_type,
                &ctx.proto_meta,
                ctx.error_name,
                ctx.struct_level_error_type,
                ctx.struct_level_error_fn,
            )
        },
        ExpectMode::None => {
            if ctx.has_default {
                let default_expr = generate_default_value(ctx.field_type, ctx.default_fn.as_deref());
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
}

fn generate_custom_type_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;
    let field_type = ctx.field_type;

    let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

    // custom types - check if proto field is optional
    if proto_is_optional {
        // proto field is optional (Option<T>), Rust field is T
        match ctx.expect_mode {
            ExpectMode::Panic => {
                quote! {
                    #field_name: proto_struct.#proto_field_ident
                        .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                        .into()
                }
            },
            ExpectMode::Error => {
                error_handler::generate_error_handling(
                    field_name,
                    &proto_field_ident,
                    field_type,
                    &ctx.proto_meta,
                    ctx.error_name,
                    ctx.struct_level_error_type,
                    ctx.struct_level_error_fn,
                )
            },
            ExpectMode::None => {
                if ctx.has_default {
                    let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
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

fn generate_default_value(field_type: &syn::Type, default_fn: Option<&str>) -> proc_macro2::TokenStream {
    if let Some(default_fn_name) = default_fn {
        let default_fn_path: syn::Path = syn::parse_str(&default_fn_name)
            .expect("Failed to parse default_fn path");
        quote! { #default_fn_path() }
    } else {
        quote! { <#field_type as Default>::default() }
    }
}

fn generate_derive_into_with_field(ctx: &FieldProcessingContext, into_with_path: &str) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;
    let into_with_path: syn::Path = syn::parse_str(&into_with_path)
        .expect("Failed to parse derive_into_with path");

    quote! {
        #proto_field_ident: #into_with_path(my_struct.#field_name)
    }
}

fn generate_transparent_from_my_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;

    let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

    if proto_is_optional {
        quote! {
            #proto_field_ident: Some(my_struct.#field_name.into())
        }
    } else {
        quote! {
            #proto_field_ident: my_struct.#field_name.into()
        }
    }
}

fn generate_option_from_my_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;
    let inner_type = type_analysis::get_inner_type_from_option(ctx.field_type).unwrap();

    if type_analysis::is_vec_type(&inner_type) {
        quote! {
            #proto_field_ident: my_struct.#field_name
                .map(|vec| vec.into_iter().map(Into::into).collect())
        }
    } else {
        quote! {
            #proto_field_ident: my_struct.#field_name.map(Into::into)
        }
    }
}

fn generate_vec_from_my_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;

    if let Some(inner_type) = type_analysis::get_inner_type_from_vec(ctx.field_type) {
        if type_analysis::is_proto_type_with_module(&inner_type, ctx.proto_module) {
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
}

fn generate_path_type_from_my_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;

    if let syn::Type::Path(type_path) = ctx.field_type {
        let is_primitive = type_analysis::is_primitive_type(ctx.field_type);
        let is_proto_type = type_path.path.segments.first()
            .is_some_and(|segment| segment.ident == ctx.proto_module);

        return if type_analysis::is_enum_type_with_explicit_attr(ctx.field_type, field) {
            generate_enum_from_my_field(ctx, field)
        } else if is_primitive {
            generate_primitive_from_my_field(ctx, field)
        } else if is_proto_type {
            generate_proto_type_from_my_field(ctx, field)
        } else {
            generate_custom_type_from_my_field(ctx, field)
        }
    }

    panic!("Only path types are supported for field '{}'", field_name);
}

fn generate_enum_from_my_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;

    let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

    if proto_is_optional {
        quote! {
            #proto_field_ident: Some(my_struct.#field_name.into())
        }
    } else {
        quote! {
            #proto_field_ident: my_struct.#field_name.into()
        }
    }
}

fn generate_primitive_from_my_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;
    let rust_is_option = type_analysis::is_option_type(ctx.field_type);

    let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

    if debug::should_output_debug(ctx.struct_name, field_name) {
        eprintln!("=== FROM_MY_FIELDS PRIMITIVE ===");
        eprintln!("  proto_is_optional: {}", proto_is_optional);
    }

    match (rust_is_option, proto_is_optional) {
        (true, false) => quote! {
            #proto_field_ident: my_struct.#field_name.unwrap_or_default()
        },
        (false, true) => quote! {
            #proto_field_ident: Some(my_struct.#field_name)
        },
        (true, true) => quote! {
            #proto_field_ident: my_struct.#field_name
        },
        (false, false) => quote! {
            #proto_field_ident: my_struct.#field_name
        },
    }
}

fn generate_proto_type_from_my_field(ctx: &FieldProcessingContext, _field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;

    quote! {
        #proto_field_ident: Some(my_struct.#field_name)
    }
}

fn generate_custom_type_from_my_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field_ident = &ctx.proto_field_ident;

    let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

    // Check if proto field is optional before wrapping in Some()
    if proto_is_optional {
        quote! {
            #proto_field_ident: Some(my_struct.#field_name.into())
        }
    } else {
        quote! {
            #proto_field_ident: my_struct.#field_name.into()
        }
    }
}