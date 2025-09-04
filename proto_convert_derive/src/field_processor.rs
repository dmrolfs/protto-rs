use super::*;
use crate::debug::CallStackDebug;
use crate::expect_analysis::ExpectMode;
use crate::optionality::FieldOptionality;
use crate::utils::maybe_option_expr;
use field_analysis::FieldProcessingContext;
use crate::field_analysis::{generate_field_conversions, ConversionStrategy};

pub fn generate_from_proto_field(
    field: &syn::Field,
    ctx: &FieldProcessingContext,
) -> proc_macro2::TokenStream {
    let _trace = CallStackDebug::with_context(
        "generate_from_proto_field",
        ctx.struct_name,
        ctx.field_name,
        &[
            ("rust_field_type", &quote!(ctx.field_type).to_string()),
            ("proto_field_ident", &ctx.proto_field_ident.to_string()),
            ("proto_name", &ctx.proto_name),
            ("proto_module", &ctx.proto_module),
        ],
    );

    let (proto_to_rust, _) = generate_field_conversions(field, ctx);

    _trace.generated_code(
        &proto_to_rust,
        ctx.struct_name,
        ctx.field_name,
        "from_proto_field_bidirectional",
        &[("conversion_direction", &"proto -> rust")],
    );

    proto_to_rust
}

pub fn generate_from_my_field(
    field: &syn::Field,
    ctx: &FieldProcessingContext,
) -> proc_macro2::TokenStream {
    let _trace = CallStackDebug::new(
        "generate_from_my_field",
        ctx.struct_name,
        ctx.field_name,
    );

    let (_, rust_to_proto) = generate_field_conversions(field, ctx);

    _trace.generated_code(
        &rust_to_proto,
        ctx.struct_name,
        ctx.field_name,
        "from_my_field_bidirectional",
        &[("conversion_direction", &"rust -> proto")],
    );

    rust_to_proto
}

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_ignored_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::new("generate_ignored_field", ctx.struct_name, ctx.field_name);
//
//     let field_name = ctx.field_name;
//     if let Some(default_fn_name) = &ctx.default_fn {
//         _trace.checkpoint_data("using default_fn", &[("function", default_fn_name)]);
//         let default_fn_path: syn::Path =
//             syn::parse_str(&default_fn_name).expect("Failed to parse default_fn function path");
//         quote! { #field_name: #default_fn_path() }
//     } else {
//         _trace.checkpoint("using Default::default()");
//         quote! { #field_name: Default::default() }
//     }
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_derive_from_with_field(
//     ctx: &FieldProcessingContext,
//     from_with_path: &str,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::with_context(
//         "generate_derive_from_with_field",
//         ctx.struct_name,
//         ctx.field_name,
//         &[("from_with_path", &from_with_path)],
//     );
//
//     let field_name = ctx.field_name;
//     let proto_field_ident = &ctx.proto_field_ident;
//     let from_with_path: syn::Path =
//         syn::parse_str(&from_with_path).expect("Failed to parse derive_from_with path");
//     quote! {
//         #field_name: #from_with_path(proto_struct.#proto_field_ident)
//     }
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_transparent_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::with_context(
//         "generate_transparent_field",
//         ctx.struct_name,
//         ctx.field_name,
//         &[
//             ("expect_mode", &format!("{:?}", ctx.expect_mode)),
//             ("has_default", &ctx.has_default.to_string()),
//         ],
//     );
//
//     let field_name = ctx.field_name;
//     let proto_field_ident = &ctx.proto_field_ident;
//     let field_type = ctx.field_type;
//
//
//     let result = match ctx.expect_mode {
//         ExpectMode::Panic => {
//             _trace.decision("ExpectMode::Panic", "use expect with panic");
//             quote! {
//                 #field_name: <#field_type>::from(
//                     proto_struct.#proto_field_ident
//                         .expect(&format!("Proto field {} is required", stringify!(#proto_field_ident)))
//                 )
//             }
//         }
//         ExpectMode::Error => {
//             _trace.decision("ExpectMode::Error", "generate error handling");
//             error_handler::generate_error_handling(
//                 field_name,
//                 &proto_field_ident,
//                 field_type,
//                 &ctx.proto_meta,
//                 ctx.error_name,
//                 ctx.struct_level_error_type,
//                 ctx.struct_level_error_fn,
//             )
//         }
//         ExpectMode::None => {
//             if ctx.has_default {
//                 _trace.decision(
//                     "ExpectMode::None + has_default",
//                     "use unwrap_or_else with default",
//                 );
//                 let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
//                 quote! {
//                     #field_name: <#field_type>::from(
//                         proto_struct.#proto_field_ident
//                             .unwrap_or_else(|| #default_expr)
//                     )
//                 }
//             } else {
//                 _trace.decision("ExpectMode::None + no_default", "direct from conversion");
//                 quote! {
//                     #field_name: <#field_type>::from(proto_struct.#proto_field_ident)
//                 }
//             }
//         }
//     };
//
//     _trace.generated_code(
//         &result,
//         ctx.struct_name,
//         field_name,
//         "transparent_field",
//         &[],
//     );
//     result
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_option_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::with_context(
//         "generate_option_field",
//         ctx.struct_name,
//         ctx.field_name,
//         &[("expect_mode", &format!("{:?}", ctx.expect_mode))],
//     );
//
//     let field_name = ctx.field_name;
//     let proto_field_ident = &ctx.proto_field_ident;
//     let field_type = ctx.field_type;
//     let inner_type = type_analysis::get_inner_type_from_option(field_type).unwrap();
//
//     let result = match ctx.expect_mode {
//         ExpectMode::Panic => {
//             _trace.decision("Panic mode", "expect with panic message");
//             quote! {
//                 #field_name: Some(proto_struct.#proto_field_ident
//                     .expect(&format!("Proto field {} is required", stringify!(#proto_field_ident)))
//                     .into())
//             }
//         }
//         ExpectMode::Error => {
//             _trace.decision("Error mode", "generate error handling");
//             error_handler::generate_error_handling(
//                 field_name,
//                 &proto_field_ident,
//                 field_type,
//                 &ctx.proto_meta,
//                 ctx.error_name,
//                 ctx.struct_level_error_type,
//                 ctx.struct_level_error_fn,
//             )
//         }
//         ExpectMode::None => {
//             if ctx.has_default {
//                 _trace.decision("None + default", "map with default fallback");
//                 let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
//                 quote! {
//                     #field_name: proto_struct.#proto_field_ident
//                         .map(#inner_type::from)
//                         .map(Some)
//                         .unwrap_or_else(|| #default_expr)
//                 }
//             } else if type_analysis::is_vec_type(&inner_type) {
//                 _trace.decision("None + vec inner", "collect iter");
//                 quote! {
//                     #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
//                 }
//             } else {
//                 _trace.decision("None + simple", "map into");
//                 quote! {
//                     #field_name: proto_struct.#proto_field_ident.map(Into::into)
//                 }
//             }
//         }
//     };
//
//     _trace.generated_code(&result, ctx.struct_name, field_name, "option_field", &[]);
//     result
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_vec_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::with_context(
//         "generate_vec_field",
//         ctx.struct_name,
//         ctx.field_name,
//         &[("has_default", &ctx.has_default.to_string())],
//     );
//
//     let field_name = ctx.field_name;
//     let proto_field_ident = &ctx.proto_field_ident;
//     let field_type = ctx.field_type;
//
//     if ctx.has_default {
//         _trace.decision("has_default", "check if empty, use default or collect");
//         let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
//         match ctx.expect_mode {
//             ExpectMode::Panic => {
//                 _trace.decision("Panic mode", "expect with panic message");
//                 quote! {
//                     #field_name: if proto_struct.#proto_field_ident.is_empty() {
//                         #default_expr
//                     } else {
//                         proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
//                     }
//                 }
//             }
//             ExpectMode::Error => {
//                 _trace.decision("Error mode", "generate error handling");
//                 error_handler::generate_error_handling(
//                     field_name,
//                     &proto_field_ident,
//                     field_type,
//                     &ctx.proto_meta,
//                     ctx.error_name,
//                     ctx.struct_level_error_type,
//                     ctx.struct_level_error_fn,
//                 )
//             }
//             ExpectMode::None => {
//                 quote! {
//                     #field_name: if proto_struct.#proto_field_ident.is_empty() {
//                         #default_expr
//                     } else {
//                         proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
//                     }
//                 }
//             }
//         }
//     } else {
//         if let Some(inner_type) = type_analysis::get_inner_type_from_vec(field_type) {
//             if type_analysis::is_proto_type(&inner_type, ctx.proto_module) {
//                 _trace.decision("no_default + proto_type", "direct assignment");
//                 quote! {
//                     #field_name: proto_struct.#proto_field_ident
//                 }
//             } else {
//                 _trace.decision("no_default + custom_type", "collect with into");
//                 quote! {
//                     #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
//                 }
//             }
//         } else {
//             _trace.decision("no_default + unknown_inner", "collect with into");
//             quote! {
//                 #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
//             }
//         }
//     }
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_path_type_field(
//     ctx: &FieldProcessingContext,
//     field: &syn::Field,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::new("generate_path_type_field", ctx.struct_name, ctx.field_name);
//
//     let field_name = ctx.field_name;
//     let field_type = ctx.field_type;
//
//     if let syn::Type::Path(type_path) = field_type {
//         let is_primitive = type_analysis::is_primitive_type(field_type);
//         let is_proto_type = type_analysis::is_proto_type(field_type, ctx.proto_module);
//         let is_custom = type_analysis::is_custom_type(field_type);
//
//         if debug::should_output_debug(ctx.struct_name, ctx.field_name) {
//             let segments_str = type_path
//                 .path
//                 .segments
//                 .iter()
//                 .map(|s| s.ident.to_string())
//                 .collect::<Vec<_>>()
//                 .join("::");
//
//             _trace.field_analysis(
//                 "PATH_TYPE_CLASSIFICATION",
//                 &[
//                     ("is_primitive", &is_primitive.to_string()),
//                     ("is_proto_type", &is_proto_type.to_string()),
//                     ("is_custom", &is_custom.to_string()),
//                     ("proto_module", ctx.proto_module),
//                     ("type_segments", &segments_str),
//                 ],
//             );
//         }
//
//         if is_primitive {
//             _trace.decision("is_primitive", "generate_primitive_field");
//             return generate_primitive_field(ctx, field);
//         } else if is_proto_type {
//             _trace.decision("is_proto_type", "generate_proto_type_field");
//             return generate_proto_type_field(ctx, field);
//         } else if is_custom {
//             _trace.decision("is_custom_type", "generate_custom_type_field");
//             return generate_custom_type_field(ctx, field);
//         } else {
//             _trace.decision("is_fallback_custom", "generate_custom_type_field");
//             return generate_custom_type_field(ctx, field);
//         }
//     }
//
//     _trace.error("Non-path type not supported");
//     panic!("Only path types are supported for field '{}'", field_name);
// }

fn apply_conditional_exprs(
    label: &str,
    ctx: &FieldProcessingContext,
    field_optionality: FieldOptionality,
    when_proto_is_optional_expr: proc_macro2::TokenStream,
    when_proto_is_required_expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let _trace = CallStackDebug::with_context(
        "apply_conditional_exprs",
        ctx.struct_name,
        ctx.field_name,
        &[
            ("label", &label),
            ("field_optionality", &format!("{:?}", field_optionality)),
        ],
    );

    let field_name = ctx.field_name;

    _trace.conditional_exprs(
        label,
        field_optionality,
        &[
            ("when_optional", &when_proto_is_optional_expr),
            ("when_required", &when_proto_is_required_expr),
        ]
    );
    // Only used for compile-time known cases now
    match field_optionality {
        FieldOptionality::Optional => {
            _trace.decision("Optional", "use optional expr - proto field is Option<T>");
            quote! { #field_name: #when_proto_is_optional_expr }
        }
        FieldOptionality::Required => {
            _trace.decision("Required", "use required expr - proto field is T");
            quote! { #field_name: #when_proto_is_required_expr }
        }
    }
}

fn apply_conditional_exprs_for_my_field(
    label: &str,
    ctx: &FieldProcessingContext,
    field_optionality: FieldOptionality,
    when_proto_field_is_optional: proc_macro2::TokenStream,
    when_proto_field_is_required: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let _trace = CallStackDebug::with_context(
        "apply_conditional_exprs_for_my_field",
        ctx.struct_name,
        ctx.field_name,
        &[
            ("label", &label),
            ("field_optionality", &format!("{:?}", field_optionality)),
        ],
    );

    let proto_field_ident = &ctx.proto_field_ident;

    // Only used for compile-time known cases now
    match field_optionality {
        FieldOptionality::Optional => {
            _trace.decision("Optional", "use optional expr - proto field is Option<T>");
            quote! { #proto_field_ident: #when_proto_field_is_optional }
        }
        FieldOptionality::Required => {
            _trace.decision("Required", "use required expr - proto field is T");
            quote! { #proto_field_ident: #when_proto_field_is_required }
        }
    }
}

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_enum_field(
//     ctx: &FieldProcessingContext,
//     field: &syn::Field,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::with_context(
//         "generate_enum_field",
//         ctx.struct_name,
//         ctx.field_name,
//         &[
//             ("expect_mode", &format!("{:?}", ctx.expect_mode)),
//             ("has_default", &ctx.has_default.to_string()),
//         ],
//     );
//
//     let field_optionality = FieldOptionality::from_field_context(ctx, field);
//     let rust_is_option = type_analysis::is_option_type(ctx.field_type);
//
//     _trace.field_analysis(
//         "ENUM_FIELD_PROCESSING",
//         &[
//             ("field_optionality", &format!("{:?}", field_optionality)),
//             ("field_type", &quote!(ctx.field_type).to_string()),
//         ],
//     );
//
//     let proto_field_ident = &ctx.proto_field_ident;
//
//     let optional_proto_field_expr = generate_optional_conversion(
//         ctx,
//         quote! { proto_struct.#proto_field_ident },
//         quote! { .into() },
//     );
//
//     let required_proto_field_expr = maybe_option_expr(
//         rust_is_option,
//         quote! { proto_struct.#proto_field_ident.into() },
//     );
//
//     apply_conditional_exprs(
//         "enum",
//         ctx,
//         field_optionality,
//         optional_proto_field_expr,
//         required_proto_field_expr,
//     )
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_primitive_field(
//     ctx: &FieldProcessingContext,
//     field: &syn::Field,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::with_context(
//         "generate_primitive_field",
//         ctx.struct_name,
//         ctx.field_name,
//         &[
//             ("has_default", &ctx.has_default.to_string()),
//             ("expect_mode", &format!("{:?}", ctx.expect_mode)),
//         ],
//     );
//
//     let field_optionality = FieldOptionality::from_field_context(ctx, field);
//     let rust_is_option = type_analysis::is_option_type(ctx.field_type);
//
//     _trace.field_analysis(
//         "PRIMITIVE_FIELD_PROCESSING",
//         &[
//             ("field_optionality", &format!("{:?}", field_optionality)),
//             ("rust_is_option", &rust_is_option.to_string()),
//         ],
//     );
//
//     let proto_field_ident = &ctx.proto_field_ident;
//
//     let optional_proto_field_expr =
//         generate_optional_conversion(ctx, quote! { proto_struct.#proto_field_ident }, quote! {});
//
//     let required_proto_field_expr =
//         maybe_option_expr(rust_is_option, quote! { proto_struct.#proto_field_ident });
//
//     apply_conditional_exprs(
//         "primitive",
//         ctx,
//         field_optionality,
//         optional_proto_field_expr,
//         required_proto_field_expr,
//     )
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_proto_type_field(
//     ctx: &FieldProcessingContext,
//     field: &syn::Field,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::new("generate_proto_type_field", ctx.struct_name, ctx.field_name);
//
//     let field_optionality = FieldOptionality::from_field_context(ctx, field);
//     let rust_is_option = type_analysis::is_option_type(ctx.field_type);
//     let proto_field_ident = &ctx.proto_field_ident;
//
//     let optional_proto_field_expr = generate_optional_conversion(
//         ctx,
//         quote! { proto_struct.#proto_field_ident },
//         quote! { .into() },
//     );
//
//     let required_proto_field_expr = maybe_option_expr(
//         rust_is_option,
//         quote! { proto_struct.#proto_field_ident.into() },
//     );
//
//     apply_conditional_exprs(
//         "proto",
//         ctx,
//         field_optionality,
//         optional_proto_field_expr,
//         required_proto_field_expr,
//     )
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_custom_type_field(
//     ctx: &FieldProcessingContext,
//     field: &syn::Field,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::new(
//         "generate_custom_type_field",
//         ctx.struct_name,
//         ctx.field_name,
//     );
//
//     let field_optionality = FieldOptionality::from_field_context(ctx, field);
//     let rust_is_option = type_analysis::is_option_type(ctx.field_type);
//
//     let proto_field_ident = &ctx.proto_field_ident;
//
//     let optional_proto_field_expr = generate_optional_conversion(
//         ctx,
//         quote! { proto_struct.#proto_field_ident },
//         quote! { .into() },
//     );
//
//     let required_proto_field_expr = maybe_option_expr(
//         rust_is_option,
//         quote! { proto_struct.#proto_field_ident.into() },
//     );
//
//     apply_conditional_exprs(
//         "custom type",
//         ctx,
//         field_optionality,
//         optional_proto_field_expr,
//         required_proto_field_expr,
//     )
// }

// fn generate_optional_conversion(
//     ctx: &FieldProcessingContext,
//     proto_field_expr: proc_macro2::TokenStream,
//     conversion_suffix: proc_macro2::TokenStream,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::new(
//         "generate_optional_conversion",
//         ctx.struct_name,
//         ctx.field_name,
//     );
//
//     let rust_is_option = type_analysis::is_option_type(ctx.field_type);
//
//     if ctx.has_default {
//         _trace.decision("has_default", "generate unwrapped conversion with default");
//         let default_expr = generate_default_value(ctx.field_type, ctx.default_fn.as_deref());
//         maybe_option_expr(
//             rust_is_option,
//             quote! {
//                 #proto_field_expr
//                     .map(|v| v #conversion_suffix)
//                     .unwrap_or_else(|| #default_expr)
//             },
//         )
//     } else {
//         _trace.decision("not has_default", "");
//         _trace.conversion_logic(
//             "optional_conversion_strategy",
//             &[
//                 ("has_default", &ctx.has_default.to_string()),
//                 ("expect_mode", &format!("{:?}", ctx.expect_mode)),
//                 ("rust_is_option", &rust_is_option.to_string()),
//             ]
//         );
//         match ctx.expect_mode {
//             ExpectMode::Panic => {
//                 _trace.decision("expect panic", "");
//                 let proto_field_ident = &ctx.proto_field_ident;
//                 maybe_option_expr(
//                     rust_is_option,
//                     quote! {
//                         #proto_field_expr
//                             .expect(&format!("Proto field {} is required", stringify!(#proto_field_ident)))
//                             #conversion_suffix
//                     },
//                 )
//             }
//             ExpectMode::Error => {
//                 _trace.decision("expect error", "");
//                 maybe_option_expr(
//                     rust_is_option,
//                     error_handler::generate_error_handling_expr(
//                         &ctx.proto_field_ident,
//                         &ctx.proto_meta,
//                         ctx.struct_level_error_fn,
//                         ctx.error_name,
//                         true,
//                     ),
//                 )
//             }
//             ExpectMode::None => {
//                 _trace.decision("no expect", "");
//                 if rust_is_option {
//                     // Rust: Option<T>, Proto: Option<T> -> direct map
//                     quote! { #proto_field_expr.map(|v| v #conversion_suffix) }
//                 } else {
//                     // Rust: T, Proto: Option<T> -> must unwrap
//                     let proto_field_ident = &ctx.proto_field_ident;
//                     quote! {
//                         #proto_field_expr
//                             .expect(&format!("Proto field {} is required", stringify!(#proto_field_ident)))
//                             #conversion_suffix
//                     }
//                 }
//             }
//         }
//     }
// }

pub fn generate_default_value(
    field_type: &syn::Type,
    default_fn: Option<&str>,
) -> proc_macro2::TokenStream {
    default_fn
        .map(|default_fn_name| {
            let default_fn_path: syn::Path = syn::parse_str(default_fn_name)
                .expect("Failed to parse default_fn path");
            quote! { #default_fn_path()}
        })
        .unwrap_or_else(|| quote! { <#field_type as Default>::default() })
}

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_derive_into_with_field(
//     ctx: &FieldProcessingContext,
//     into_with_path: &str,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::with_context(
//         "generate_derive_into_with_field",
//         ctx.struct_name,
//         ctx.field_name,
//         &[("into_with_path", &into_with_path)],
//     );
//
//     let field_name = ctx.field_name;
//     let proto_field_ident = &ctx.proto_field_ident;
//     let into_with_path: syn::Path =
//         syn::parse_str(&into_with_path).expect("Failed to parse derive_into_with path");
//
//     quote! {
//         #proto_field_ident: #into_with_path(my_struct.#field_name)
//     }
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_transparent_from_my_field(
//     ctx: &FieldProcessingContext,
//     field: &syn::Field,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::new(
//         "generate_transparent_from_my_field",
//         ctx.struct_name,
//         ctx.field_name,
//     );
//
//     let field_name = ctx.field_name;
//     let field_optionality = FieldOptionality::from_field_context(ctx, field);
//     let rust_is_option = type_analysis::is_option_type(ctx.field_type);
//
//     _trace.field_analysis(
//         "TRANSPARENT FROM_MY_FIELD",
//         &[
//             ("rust_is_option", &rust_is_option.to_string()),
//             ("field_optionality", &format!("{:?}", field_optionality)),
//         ],
//     );
//
//     let optional_proto_field_expr = quote! {
//         Some(my_struct.#field_name.into())
//     };
//
//     let required_proto_field_expr = quote! {
//         my_struct.#field_name.into()
//     };
//
//     apply_conditional_exprs_for_my_field(
//         "transparent",
//         ctx,
//         field_optionality,
//         optional_proto_field_expr,
//         required_proto_field_expr,
//     )
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_option_from_my_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::new(
//         "generate_option_from_my_field",
//         ctx.struct_name,
//         ctx.field_name,
//     );
//
//     let field_name = ctx.field_name;
//     let proto_field_ident = &ctx.proto_field_ident;
//     let inner_type = type_analysis::get_inner_type_from_option(ctx.field_type).unwrap();
//
//     if type_analysis::is_vec_type(&inner_type) {
//         _trace.decision("inner_type is Vec", "map with collect");
//         quote! {
//             #proto_field_ident: my_struct.#field_name
//                 .map(|vec| vec.into_iter().map(Into::into).collect())
//         }
//     } else {
//         _trace.decision("inner_type is simple", "map with into");
//         quote! {
//             #proto_field_ident: my_struct.#field_name.map(Into::into)
//         }
//     }
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_vec_from_my_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::new(
//         "generate_vec_from_my_field",
//         ctx.struct_name,
//         ctx.field_name,
//     );
//
//     let field_name = ctx.field_name;
//     let proto_field_ident = &ctx.proto_field_ident;
//
//     if let Some(inner_type) = type_analysis::get_inner_type_from_vec(ctx.field_type) {
//         if type_analysis::is_proto_type(&inner_type, ctx.proto_module) {
//             _trace.decision("inner is proto type", "direct assignment");
//             quote! {
//                 #proto_field_ident: my_struct.#field_name
//             }
//         } else {
//             _trace.decision("inner is custom type", "collect with into");
//             quote! {
//                 #proto_field_ident: my_struct.#field_name.into_iter().map(Into::into).collect()
//             }
//         }
//     } else {
//         _trace.decision("unknown inner type", "collect with into");
//         quote! {
//             #proto_field_ident: my_struct.#field_name.into_iter().map(Into::into).collect()
//         }
//     }
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_path_type_from_my_field(
//     ctx: &FieldProcessingContext,
//     field: &syn::Field,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::new(
//         "generate_path_type_from_my_field",
//         ctx.struct_name,
//         ctx.field_name,
//     );
//
//     if let syn::Type::Path(type_path) = ctx.field_type {
//         let is_primitive = type_analysis::is_primitive_type(ctx.field_type);
//         let is_proto_type = type_analysis::is_proto_type(ctx.field_type, ctx.proto_module);
//
//         if is_primitive {
//             _trace.decision("is_primitive", "generate_primitive_from_my_field");
//             return generate_primitive_from_my_field(ctx, field);
//         } else if is_proto_type {
//             _trace.decision("is_proto_type", "generate_proto_type_from_my_field");
//             return generate_proto_type_from_my_field(ctx, field);
//         } else {
//             _trace.decision("is_custom_type", "generate_custom_type_from_my_field");
//             return generate_custom_type_from_my_field(ctx, field);
//         };
//     }
//
//     _trace.error("Non-path type not supported");
//     panic!(
//         "Only path types are supported for field '{}'",
//         ctx.field_name
//     );
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_enum_from_my_field(
//     ctx: &FieldProcessingContext,
//     field: &syn::Field,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::with_context(
//         "generate_enum_from_my_field",
//         ctx.struct_name,
//         ctx.field_name,
//         &[(
//             "rust_is_option",
//             &type_analysis::is_option_type(ctx.field_type).to_string(),
//         )],
//     );
//
//     let field_optionality = FieldOptionality::from_field_context(ctx, field);
//     let rust_is_option = type_analysis::is_option_type(ctx.field_type);
//
//     _trace.field_analysis(
//         "ENUM FROM_MY_FIELD",
//         &[
//             ("rust_is_option", &rust_is_option.to_string()),
//             ("field_optionality", &format!("{:?}", field_optionality)),
//         ],
//     );
//
//     let field_name = ctx.field_name;
//
//     let optional_proto_field = if rust_is_option {
//         quote! { my_struct.#field_name.map(|v| v.into()) }
//     } else {
//         quote! { Some(my_struct.#field_name.into()) }
//     };
//
//     let required_proto_field = if rust_is_option {
//         quote! { my_struct.#field_name.unwrap_or_default().into() }
//     } else {
//         quote! { my_struct.#field_name.into() }
//     };
//
//     apply_conditional_exprs_for_my_field(
//         "enum_from_my_field",
//         ctx,
//         field_optionality,
//         optional_proto_field,
//         required_proto_field,
//     )
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_primitive_from_my_field(
//     ctx: &FieldProcessingContext,
//     field: &syn::Field,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::with_context(
//         "generate_primitive_from_my_field",
//         ctx.struct_name,
//         ctx.field_name,
//         &[(
//             "rust_is_option",
//             &type_analysis::is_option_type(ctx.field_type).to_string(),
//         )],
//     );
//
//     let field_optionality = FieldOptionality::from_field_context(ctx, field);
//     let rust_is_option = type_analysis::is_option_type(ctx.field_type);
//
//     _trace.field_analysis(
//         "PRIMITIVE FROM_MY_FIELD",
//         &[
//             ("rust_is_option", &rust_is_option.to_string()),
//             ("field_optionality", &format!("{:?}", field_optionality)),
//         ],
//     );
//
//     let field_name = ctx.field_name;
//
//     let optional_proto_field = if rust_is_option {
//         quote! { my_struct.#field_name }
//     } else {
//         quote! { Some(my_struct.#field_name) }
//     };
//
//     let required_proto_field = if rust_is_option {
//         quote! { my_struct.#field_name.unwrap_or_default() }
//     } else {
//         quote! { my_struct.#field_name }
//     };
//
//     apply_conditional_exprs_for_my_field(
//         "primitive",
//         ctx,
//         field_optionality,
//         optional_proto_field,
//         required_proto_field,
//     )
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_proto_type_from_my_field(
//     ctx: &FieldProcessingContext,
//     _field: &syn::Field,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::new(
//         "generate_proto_type_from_my_field",
//         ctx.struct_name,
//         ctx.field_name,
//     );
//
//     let field_name = ctx.field_name;
//     let proto_field_ident = &ctx.proto_field_ident;
//
//     _trace.checkpoint("proto type -> wrap in Some");
//     quote! {
//         #proto_field_ident: Some(my_struct.#field_name)
//     }
// }

// #[deprecated(note = "Use generate_field_conversions from field_analysis instead")]
// fn generate_custom_type_from_my_field(
//     ctx: &FieldProcessingContext,
//     field: &syn::Field,
// ) -> proc_macro2::TokenStream {
//     let _trace = CallStackDebug::new(
//         "generate_custom_type_from_my_field",
//         ctx.struct_name,
//         ctx.field_name,
//     );
//
//     let field_optionality = FieldOptionality::from_field_context(ctx, field);
//     let rust_is_option = type_analysis::is_option_type(ctx.field_type);
//
//     _trace.field_analysis(
//         "ENUM FROM_MY_FIELD",
//         &[
//             ("rust_is_option", &rust_is_option.to_string()),
//             ("field_optionality", &field_optionality.to_string()),
//         ]
//     );
//
//     let field_name = ctx.field_name;
//
//     let optional_proto_field_expr = if rust_is_option {
//         quote! { my_struct.#field_name.map(|v| v.into()) }
//     } else {
//         quote! { Some(my_struct.#field_name.into()) }
//     };
//
//     let required_proto_field_expr = if rust_is_option {
//         quote! { my_struct.#field_name.unwrap_or_default().into() }
//     } else {
//         quote! { my_struct.#field_name.into() }
//     };
//
//     apply_conditional_exprs_for_my_field(
//         "custom_type_from_my_field",
//         ctx,
//         field_optionality,
//         optional_proto_field_expr,
//         required_proto_field_expr,
//     )
// }
