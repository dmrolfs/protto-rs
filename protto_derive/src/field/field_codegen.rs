use crate::analysis::type_analysis;
use crate::debug::CallStackDebug;
use crate::field::{
    FieldProcessingContext,
    conversion_strategy::{
        CollectionStrategy, DirectStrategy, FieldConversionStrategy, OptionStrategy,
    },
    custom_conversion::CustomConversionStrategy,
    error_mode::ErrorMode,
    info::{ProtoFieldInfo, RustFieldInfo},
};
use quote::quote;

impl FieldConversionStrategy {
    /// Generate proto->rust conversion code using new simplified logic
    pub fn generate_proto_to_rust_conversion(
        &self,
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        rust_field_info: &RustFieldInfo,
        proto_field_info: &ProtoFieldInfo,
    ) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field = &ctx.proto_field_ident;

        match self {
            Self::Ignore => generate_ignore_proto_to_rust(ctx),

            Self::Custom(custom_strategy) => generate_custom_proto_to_rust(
                custom_strategy,
                field,
                field_name,
                proto_field,
                None,
                ctx,
            ),

            Self::CustomWithError(custom_strategy, error_mode) => generate_custom_proto_to_rust(
                custom_strategy,
                field,
                field_name,
                proto_field,
                Some(error_mode),
                ctx,
            ),

            Self::Direct(direct_strategy) => {
                generate_direct_proto_to_rust(direct_strategy, field_name, proto_field)
            }

            Self::Option(option_strategy) => generate_option_proto_to_rust(
                option_strategy,
                field_name,
                proto_field,
                ctx,
                rust_field_info,
                proto_field_info,
            ),

            Self::Transparent(error_mode) => {
                if proto_field_info.is_optional() {
                    generate_transparent_proto_to_rust(
                        error_mode,
                        ctx,
                        rust_field_info,
                        proto_field_info,
                    )
                } else {
                    let field_name = ctx.field_name;
                    let proto_field = &ctx.proto_field_ident;
                    let field_type = ctx.field_type;
                    quote! { #field_name: #field_type::from(proto_struct.#proto_field) }
                }
            }

            Self::Collection(collection_strategy) => {
                generate_collection_proto_to_rust(collection_strategy, ctx)
            }
        }
    }

    /// Generate rust->proto conversion code using new simplified logic
    pub fn generate_rust_to_proto_conversion(
        &self,
        ctx: &FieldProcessingContext,
        _field: &syn::Field,
        rust_field_info: &RustFieldInfo,
        proto_field_info: &ProtoFieldInfo,
    ) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field = &ctx.proto_field_ident;

        match self {
            Self::Ignore => {
                // Ignored fields are not included in proto struct
                quote! { /* field ignored */ }
            }

            Self::Custom(custom_strategy) | Self::CustomWithError(custom_strategy, _) => {
                generate_custom_rust_to_proto(
                    custom_strategy,
                    field_name,
                    proto_field,
                    rust_field_info,
                    proto_field_info,
                )
            }

            Self::Direct(direct_strategy) => {
                generate_direct_rust_to_proto(direct_strategy, field_name, proto_field)
            }

            Self::Option(option_strategy) => generate_option_rust_to_proto(
                option_strategy,
                field_name,
                proto_field,
                rust_field_info,
                proto_field_info,
            ),

            Self::Transparent(error_mode) => generate_transparent_rust_to_proto(
                error_mode,
                field_name,
                proto_field,
                rust_field_info,
                proto_field_info,
            ),

            Self::Collection(collection_strategy) => {
                generate_collection_rust_to_proto(collection_strategy, field_name, proto_field)
            }
        }
    }
}

// -- Proto-to-Rust generation functions --
fn generate_ignore_proto_to_rust(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;

    if let Some(default_fn_name) = &ctx.default_fn {
        let default_fn_path: syn::Path =
            syn::parse_str(default_fn_name).expect("Failed to parse default_fn function path");
        quote! { #field_name: #default_fn_path() }
    } else {
        quote! { #field_name: Default::default() }
    }
}

fn generate_custom_proto_to_rust(
    custom_strategy: &CustomConversionStrategy,
    field: &syn::Field,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
    _error_mode: Option<&ErrorMode>,
    ctx: &FieldProcessingContext,
) -> proc_macro2::TokenStream {
    let rust_field_info = RustFieldInfo::analyze(ctx, field);
    let proto_field_info = ProtoFieldInfo::infer_from(ctx, field, &rust_field_info);

    match custom_strategy {
        CustomConversionStrategy::FromFn(fn_path)
        | CustomConversionStrategy::Bidirectional(fn_path, _) => {
            let from_fn: syn::Path =
                syn::parse_str(fn_path).expect("Failed to parse function path");

            if proto_field_info.is_repeated() {
                quote! { #field_name: #from_fn(proto_struct.#proto_field) }
            } else if proto_field_info.is_optional() {
                if rust_field_info.is_option {
                    quote! { #field_name: #from_fn(proto_struct.#proto_field) }
                } else {
                    quote! {
                        #field_name: #from_fn(
                            proto_struct.#proto_field
                                .expect(&format!(
                                    "Proto field {} is required for custom conversion",
                                    stringify!(#proto_field)
                                ))
                        )
                    }
                }
            } else {
                quote! { #field_name: #from_fn(proto_struct.#proto_field) }
            }
        }
        CustomConversionStrategy::IntoFn(_) => {
            // Fallback to .into() for proto->rust when only rust->proto function provided
            quote! { #field_name: proto_struct.#proto_field.into() }
        }
    }
}

fn generate_direct_proto_to_rust(
    direct_strategy: &DirectStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    match direct_strategy {
        DirectStrategy::Assignment => {
            quote! { #field_name: proto_struct.#proto_field }
        }
        DirectStrategy::WithConversion => {
            quote! { #field_name: proto_struct.#proto_field.into() }
        }
    }
}

fn generate_option_proto_to_rust(
    option_strategy: &OptionStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
    ctx: &FieldProcessingContext,
    rust_field_info: &RustFieldInfo,
    proto_field_info: &ProtoFieldInfo,
) -> proc_macro2::TokenStream {
    let _trace = CallStackDebug::new(
        "field::field_codegen",
        "generate_option_proto_to_rust",
        "",
        "",
    );
    match option_strategy {
        OptionStrategy::Wrap => {
            _trace.decision("wrap_option", "wrap field in Some()");
            quote! { #field_name: Some(proto_struct.#proto_field.into()) }
        }
        OptionStrategy::Unwrap(error_mode) => {
            _trace.decision("unwrap_option", "unwrap field considering error mode");
            generate_unwrap_with_error_mode(
                error_mode,
                field_name,
                proto_field,
                ctx,
                rust_field_info,
                proto_field_info,
            )
        }
        OptionStrategy::Map => {
            _trace.decision("map_option", "unwrap field and map");
            quote! { #field_name: proto_struct.#proto_field.map(|v| v.into()) }
        }
    }
}

fn generate_transparent_proto_to_rust(
    error_mode: &ErrorMode,
    ctx: &FieldProcessingContext,
    _rust_field_info: &RustFieldInfo,
    proto_field_info: &ProtoFieldInfo,
) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field = &ctx.proto_field_ident;
    let field_type = ctx.field_type;

    if proto_field_info.is_optional() {
        let get_error_type = || -> syn::Ident {
            let struct_name = &ctx.struct_name;
            let error_type_name = format!("{}ConversionError", struct_name);
            syn::parse_str(&error_type_name).expect("Failed to parse error type name")
        };

        let error_message = quote! {
            &format!("Proto field {} is required for transparent conversion", stringify!(#proto_field))
        };

        // Check if the Rust field is Option<TransparentWrapper>
        if let Some(inner_type) = type_analysis::get_inner_type_from_option(field_type) {
            // This is Option<TransparentWrapper> -> proto_optional
            match error_mode {
                ErrorMode::None => {
                    quote! { #field_name: proto_struct.#proto_field.map(#inner_type::from) }
                }
                ErrorMode::Panic => {
                    quote! {
                        #field_name: Some(#inner_type::from(
                            proto_struct.#proto_field.expect(#error_message)
                        ))
                    }
                }
                ErrorMode::Error => {
                    let error_type = get_error_type();
                    quote! {
                        #field_name: #inner_type::from(
                            proto_struct.#proto_field
                                .ok_or_else(|| #error_type::MissingField(stringify!(#proto_field).to_string()))?  // DMR: Use struct-specific error type
                        )
                    }
                }
                ErrorMode::Default(default_fn) => {
                    let default_expr = generate_default_expr(default_fn);
                    quote! {
                        #field_name: proto_struct.#proto_field
                            .map(#inner_type::from)
                            .or_else(|| Some(#default_expr))
                    }
                }
            }
        } else {
            let conversion_expr = generate_conversion_expr(
                error_mode,
                proto_field,
                field_type,
                &get_error_type,
                &error_message,
            );
            quote! { #field_name: #field_type::from(#conversion_expr) }
        }
    } else {
        quote! { #field_name: #field_type::from(proto_struct.#proto_field) }
    }
}

fn generate_conversion_expr(
    error_mode: &ErrorMode,
    proto_field: &syn::Ident,
    field_type: &syn::Type,
    get_error_type: &dyn Fn() -> syn::Ident,
    error_message: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match error_mode {
        ErrorMode::None | ErrorMode::Panic => {
            quote! { proto_struct.#proto_field.expect(#error_message) }
        }
        ErrorMode::Error => {
            let error_type = get_error_type();
            quote! {
                proto_struct.#proto_field
                    .ok_or_else(|| #error_type::MissingField(stringify!(#proto_field).to_string()))?
            }
        }
        ErrorMode::Default(Some(default_fn)) => {
            let default_fn_path: syn::Path =
                syn::parse_str(default_fn).expect("Failed to parse default function");
            quote! {
                proto_struct.#proto_field.unwrap_or_else(|| {
                    let default_val: #field_type = #default_fn_path();
                    default_val.into()
                })
            }
        }
        ErrorMode::Default(None) => {
            quote! { proto_struct.#proto_field.unwrap_or_default() }
        }
    }
}

fn generate_default_expr(default_fn: &Option<String>) -> proc_macro2::TokenStream {
    default_fn
        .as_ref()
        .map(|default_fn| {
            let default_fn_path: syn::Path =
                syn::parse_str(default_fn).expect("Failed to parse default function");
            quote! { #default_fn_path() }
        })
        .unwrap_or_else(|| quote! { Default::default() })
}

fn generate_collection_proto_to_rust(
    collection_strategy: &CollectionStrategy,
    ctx: &FieldProcessingContext,
) -> proc_macro2::TokenStream {
    let field_name = ctx.field_name;
    let proto_field = &ctx.proto_field_ident;

    match collection_strategy {
        CollectionStrategy::Collect(error_mode) => match error_mode {
            ErrorMode::Default(Some(default_fn)) => {
                let default_fn_path: syn::Path =
                    syn::parse_str(default_fn).expect("Failed to parse default function");
                quote! {
                    #field_name: if proto_struct.#proto_field.is_empty() {
                        #default_fn_path()
                    } else {
                        proto_struct.#proto_field.into_iter().map(Into::into).collect()
                    }
                }
            }
            ErrorMode::Default(None) => {
                quote! {
                    #field_name: if proto_struct.#proto_field.is_empty() {
                        Default::default()
                    } else {
                        proto_struct.#proto_field.into_iter().map(Into::into).collect()
                    }
                }
            }
            ErrorMode::Error if ctx.default_fn.is_some() => {
                let default_fn_path: syn::Path = ctx
                    .default_fn
                    .as_ref()
                    .and_then(|default_fn| syn::parse_str(default_fn).ok())
                    .expect("Failed to parse default function");
                quote! {
                    #field_name: if proto_struct.#proto_field.is_empty() {
                        #default_fn_path()
                    } else {
                        proto_struct.#proto_field.into_iter().map(Into::into).collect()
                    }
                }
            }
            ErrorMode::Error if ctx.proto_meta.error_fn.is_some() => {
                let error_fn_path: syn::Path = ctx
                    .proto_meta
                    .error_fn
                    .as_ref()
                    .and_then(|error_fn| syn::parse_str(error_fn).ok())
                    .expect("Failed to parse error function");
                quote! {
                    #field_name: if proto_struct.#proto_field.is_empty() {
                        return Err(#error_fn_path(stringify!(#proto_field)));
                    } else {
                        proto_struct.#proto_field.into_iter().map(Into::into).collect()
                    }
                }
            }
            ErrorMode::Error => {
                quote! {
                    #field_name: proto_struct.#proto_field.into_iter().map(Into::into).collect()
                }
            }
            ErrorMode::Panic | ErrorMode::None => {
                quote! {
                    #field_name: proto_struct.#proto_field.into_iter().map(Into::into).collect()
                }
            }
        },
        CollectionStrategy::MapOption => {
            // Check if rust field is Option<Vec<T>> -> handle empty vec as None
            if is_option_vec_type(ctx.field_type) {
                quote! {
                    #field_name: if proto_struct.#proto_field.is_empty() {
                        None
                    } else {
                        Some(proto_struct.#proto_field.into_iter().map(Into::into).collect())
                    }
                }
            } else {
                // Option<Vec<T>> case where we map the option
                quote! {
                    #field_name: proto_struct.#proto_field
                        .map(|vec| vec.into_iter().map(Into::into).collect())
                }
            }
        }
        CollectionStrategy::DirectAssignment => {
            quote! { #field_name: proto_struct.#proto_field }
        }
    }
}

// -- Rust-to-Proto generation functions --
fn generate_custom_rust_to_proto(
    custom_strategy: &CustomConversionStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
    rust_field_info: &RustFieldInfo,
    proto_field_info: &ProtoFieldInfo,
) -> proc_macro2::TokenStream {
    match custom_strategy {
        CustomConversionStrategy::IntoFn(fn_path)
        | CustomConversionStrategy::Bidirectional(_, fn_path) => {
            let into_fn: syn::Path =
                syn::parse_str(fn_path).expect("Failed to parse function path");

            if proto_field_info.is_optional() && !rust_field_info.is_option {
                quote! { #proto_field: Some(#into_fn(my_struct.#field_name)) }
            } else {
                quote! { #proto_field: #into_fn(my_struct.#field_name) }
            }
        }
        CustomConversionStrategy::FromFn(_) => {
            // Fallback to .into() for rust->proto when only proto->rust function provided
            if proto_field_info.is_optional() {
                quote! { #proto_field: Some(my_struct.#field_name.into()) }
            } else {
                quote! { #proto_field: my_struct.#field_name.into() }
            }
        }
    }
}

fn generate_direct_rust_to_proto(
    direct_strategy: &DirectStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    match direct_strategy {
        DirectStrategy::Assignment => {
            quote! { #proto_field: my_struct.#field_name }
        }
        DirectStrategy::WithConversion => {
            quote! { #proto_field: my_struct.#field_name.into() }
        }
    }
}

fn generate_option_rust_to_proto(
    option_strategy: &OptionStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
    rust_field_info: &RustFieldInfo,
    proto_field_info: &ProtoFieldInfo,
) -> proc_macro2::TokenStream {
    match option_strategy {
        OptionStrategy::Wrap => {
            quote! { #proto_field: Some(my_struct.#field_name.into()) }
        }
        OptionStrategy::Unwrap(_)
            if rust_field_info.is_option && proto_field_info.is_optional() =>
        {
            quote! { #proto_field: my_struct.#field_name.map(|v| v.into()) }
        }
        OptionStrategy::Unwrap(_) => {
            quote! { #proto_field: Some(my_struct.#field_name.into()) }
        }
        OptionStrategy::Map => {
            quote! { #proto_field: my_struct.#field_name.map(|v| v.into()) }
        }
    }
}

fn generate_transparent_rust_to_proto(
    _error_mode: &ErrorMode,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
    rust_field_info: &RustFieldInfo,
    proto_field_info: &ProtoFieldInfo,
) -> proc_macro2::TokenStream {
    if proto_field_info.is_optional() {
        type_analysis::get_inner_type_from_option(&rust_field_info.field_type)
            .map(|_inner_type| {
                quote! { #proto_field: my_struct.#field_name.map(|inner| inner.into()) }
            })
            .unwrap_or_else(|| quote! { #proto_field: Some(my_struct.#field_name.into()) })
    } else {
        quote! { #proto_field: my_struct.#field_name.into() }
    }
}

fn generate_collection_rust_to_proto(
    collection_strategy: &CollectionStrategy,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
) -> proc_macro2::TokenStream {
    match collection_strategy {
        CollectionStrategy::Collect(_) => {
            quote! {
                #proto_field: my_struct.#field_name.into_iter().map(Into::into).collect()
            }
        }
        CollectionStrategy::MapOption => {
            quote! {
                #proto_field: my_struct.#field_name
                    .map(|vec| vec.into_iter().map(Into::into).collect())
                    .unwrap_or_default()
            }
        }
        CollectionStrategy::DirectAssignment => {
            quote! { #proto_field: my_struct.#field_name }
        }
    }
}

// -- Helper functions --
fn generate_unwrap_with_error_mode(
    error_mode: &ErrorMode,
    field_name: &syn::Ident,
    proto_field: &syn::Ident,
    ctx: &FieldProcessingContext,
    rust_field_info: &RustFieldInfo,
    proto_field_info: &ProtoFieldInfo,
) -> proc_macro2::TokenStream {
    let _trace = CallStackDebug::new(
        "field::field_codegen",
        "generate_unwrap_with_error_mode",
        "",
        "",
    );

    let get_context_error_fn = |c: &FieldProcessingContext| {
        c.proto_meta
            .error_fn
            .as_ref()
            .and_then(|error_fn| syn::parse_str::<syn::Path>(error_fn).ok())
            .expect("Failed to parse error function path")
    };
    let derive_struct_error_type = |c: &FieldProcessingContext| {
        let error_type_name = format!("{}ConversionError", c.struct_name);
        syn::parse_str::<syn::Ident>(&error_type_name).expect("Failed to parse error type name")
    };

    match error_mode {
        ErrorMode::None | ErrorMode::Panic => {
            _trace.decision("unwrap_with_expect", "Required field with panic on missing");
            quote! {
                #field_name: proto_struct.#proto_field
                    .expect(&format!("Proto field {} is required", stringify!(#proto_field)))
                    .into()
            }
        }

        ErrorMode::Error
            if ctx.proto_meta.error_fn.is_some()
                && rust_field_info.is_option
                && proto_field_info.is_optional() =>
        {
            _trace.decision(
                "optional_with_custom_error",
                "Option<T> -> Option<T> with custom error function",
            );
            let error_fn = get_context_error_fn(ctx);
            quote! {
                #field_name: Some(proto_struct.#proto_field
                    .ok_or_else(|| #error_fn(stringify!(#proto_field)))?
                    .into())
            }
        }
        ErrorMode::Error if ctx.proto_meta.error_fn.is_some() => {
            _trace.decision(
                "unwrap_with_custom_error",
                "Required field with custom error function",
            );
            let error_fn = get_context_error_fn(ctx);
            quote! {
                #field_name: proto_struct.#proto_field
                    .ok_or_else(|| #error_fn(stringify!(#proto_field)))?
                    .into()
            }
        }
        ErrorMode::Error if rust_field_info.is_option && proto_field_info.is_optional() => {
            _trace.decision(
                "optional_with_generated_error",
                "Option<T> -> Option<T> with generated error type",
            );
            let error_type = derive_struct_error_type(ctx);
            quote! {
                #field_name: Some(proto_struct.#proto_field
                    .ok_or_else(|| #error_type::MissingField(stringify!(#proto_field).to_string()))?
                    .into())
            }
        }
        ErrorMode::Error => {
            _trace.decision(
                "unwrap_with_generated_error",
                "Required field with generated error type",
            );
            let error_type = derive_struct_error_type(ctx);
            quote! {
                #field_name: proto_struct.#proto_field
                    .ok_or_else(|| #error_type::MissingField(stringify!(#proto_field).to_string()))?
                    .into()
            }
        }

        ErrorMode::Default(Some(default_fn)) if rust_field_info.is_option => {
            let default_fn: syn::Path =
                syn::parse_str(default_fn).expect("Failed to parse default function");
            quote! {
                #field_name: proto_struct.#proto_field
                    .map(|v| v.into())
                    .or_else(|| #default_fn())
            }
        }
        ErrorMode::Default(Some(default_fn)) => {
            _trace.decision(
                "optional_with_default_fn",
                "Option<T> field with custom default function",
            );
            let default_fn: syn::Path =
                syn::parse_str(default_fn).expect("Failed to parse default function");
            quote! {
                #field_name: proto_struct.#proto_field
                    .map(|v| v.into())
                    .unwrap_or_else(|| #default_fn())
            }
        }
        ErrorMode::Default(None) => {
            _trace.decision("unwrap_with_default_trait", "Field with Default trait");
            quote! {
                #field_name: proto_struct.#proto_field
                    .map(|v| v.into())
                    .unwrap_or_default()
            }
        }
    }
}

fn is_option_vec_type(field_type: &syn::Type) -> bool {
    type_analysis::get_inner_type_from_option(field_type)
        .map(|inner| type_analysis::is_vec_type(&inner))
        .unwrap_or(false)
}

#[cfg(test)]
pub mod test_helpers {
    use crate::field::FieldProcessingContext;
    use syn::parse::Parser;

    /// Create a mock field processing context for testing
    pub fn create_mock_context(
        struct_name: &str,
        field_name: &str,
        field_type: &str,
        proto_module: &str,
        attributes: &[&str],
    ) -> (syn::Field, FieldProcessingContext<'static>) {
        // Parse field type
        let field_type: syn::Type = syn::parse_str(field_type).unwrap();

        // Create attributes from string descriptions
        let mut attrs = Vec::new();
        for attr_str in attributes {
            if !attr_str.is_empty() {
                let attr_tokens: proc_macro2::TokenStream =
                    format!("#[protto({})]", attr_str).parse().unwrap();
                let attrs_parsed: Vec<syn::Attribute> =
                    syn::Attribute::parse_outer.parse2(attr_tokens).unwrap();
                attrs.extend(attrs_parsed);
            }
        }

        // Create field
        let field: syn::Field = syn::Field {
            attrs,
            vis: syn::Visibility::Public(Default::default()),
            mutability: syn::FieldMutability::None,
            ident: Some(syn::Ident::new(field_name, proc_macro2::Span::call_site())),
            colon_token: Some(syn::Token![:](proc_macro2::Span::call_site())),
            ty: field_type.clone(),
        };

        let field_static: &'static syn::Field = Box::leak(field.clone().into());

        // Create leaked strings for 'static lifetime in tests
        let struct_name_static = Box::leak(struct_name.to_string().into_boxed_str());
        let proto_module_static = Box::leak(proto_module.to_string().into_boxed_str());
        let proto_name_static = Box::leak(struct_name.to_string().into_boxed_str());

        // Create identifiers
        let struct_ident =
            Box::leak(syn::Ident::new(struct_name_static, proc_macro2::Span::call_site()).into());
        let error_ident =
            Box::leak(syn::Ident::new("TestError", proc_macro2::Span::call_site()).into());

        // Use FieldProcessingContext::new constructor
        let context = FieldProcessingContext::new(
            struct_ident,
            field_static,
            error_ident,
            &None, // struct_level_error_fn
            proto_module_static,
            proto_name_static,
        );

        (field, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::optionality::FieldOptionality;
    use crate::debug;
    use crate::field::info::ProtoMapping;

    #[test]
    fn test_code_generation_produces_valid_tokens() {
        // Test that all strategy types produce valid TokenStream output
        let test_cases = vec![
            ("ignore_field", FieldConversionStrategy::Ignore),
            (
                "direct_assignment",
                FieldConversionStrategy::Direct(DirectStrategy::Assignment),
            ),
            (
                "direct_conversion",
                FieldConversionStrategy::Direct(DirectStrategy::WithConversion),
            ),
        ];

        for (name, strategy) in test_cases {
            let (field, ctx) = test_helpers::create_mock_context(
                "TestStruct",
                "test_field",
                "String",
                "proto",
                &[],
            );
            let rust_field_info = RustFieldInfo::analyze(&ctx, &field);
            let proto_field_info = ProtoFieldInfo::infer_from(&ctx, &field, &rust_field_info);

            let proto_to_rust = strategy.generate_proto_to_rust_conversion(
                &ctx,
                &field,
                &rust_field_info,
                &proto_field_info,
            );
            let rust_to_proto = strategy.generate_rust_to_proto_conversion(
                &ctx,
                &field,
                &rust_field_info,
                &proto_field_info,
            );

            // Verify tokens are not empty and parse correctly
            assert!(
                !proto_to_rust.is_empty(),
                "Proto->Rust generation failed for {}",
                name
            );
            assert!(
                !rust_to_proto.is_empty() || matches!(strategy, FieldConversionStrategy::Ignore),
                "Rust->Proto generation failed for {}",
                name
            );

            // Verify the generated code contains the field name
            let proto_to_rust_str = proto_to_rust.to_string();
            let rust_to_proto_str = rust_to_proto.to_string();

            assert!(
                proto_to_rust_str.contains("test_field"),
                "Generated proto->rust code should contain field name: {}",
                proto_to_rust_str
            );

            if !matches!(strategy, FieldConversionStrategy::Ignore) {
                assert!(
                    rust_to_proto_str.contains("test_field"),
                    "Generated rust->proto code should contain field name: {}",
                    rust_to_proto_str
                );
            }
        }
    }

    #[test]
    fn test_error_mode_code_generation() {
        let error_modes = vec![
            ErrorMode::None,
            ErrorMode::Panic,
            ErrorMode::Default(None),
            ErrorMode::Default(Some("test_default".to_string())),
        ];

        for error_mode in error_modes {
            let strategy = FieldConversionStrategy::Transparent(error_mode.clone());

            let (field, ctx) = test_helpers::create_mock_context(
                "TestStruct",
                "test_field",
                "Option<TransparentWrapper>",
                "proto",
                &["transparent"], // Remove expect attributes
            );

            let rust_field_info = RustFieldInfo::analyze(&ctx, &field);
            let proto_field_info = ProtoFieldInfo::infer_from(&ctx, &field, &rust_field_info);
            let proto_field_info = ProtoFieldInfo {
                type_name: proto_field_info.type_name,
                mapping: proto_field_info.mapping,
                optionality: FieldOptionality::Optional,
            };

            let proto_to_rust = strategy.generate_proto_to_rust_conversion(
                &ctx,
                &field,
                &rust_field_info,
                &proto_field_info,
            );
            let code_str = proto_to_rust.to_string();
            println!("Generated code for {:?}: {}", error_mode, code_str); // DMR: Debug print

            match &error_mode {
                ErrorMode::Panic => {
                    assert!(code_str.contains("expect"), "Panic mode should use expect");
                    assert!(
                        code_str.contains("Some"),
                        "Should wrap result in Some for Option field"
                    );
                }
                ErrorMode::Default(Some(_)) => {
                    assert!(
                        code_str.contains("test_default"),
                        "Should use custom default"
                    );
                }
                ErrorMode::Default(None) => {
                    assert!(
                        code_str.contains("Default :: default"),
                        "Should use Default trait"
                    );
                    assert!(
                        code_str.contains("or_else"),
                        "Should use or_else for Option handling"
                    );
                }
                _ => {} // Other modes have different patterns
            }
        }
    }

    #[test]
    fn test_transparent_optional_error_mode_code_generation() {
        let error_modes = vec![
            ErrorMode::None,
            ErrorMode::Panic,
            ErrorMode::Default(None),
            ErrorMode::Default(Some("test_default".to_string())),
        ];

        for error_mode in error_modes {
            let strategy = FieldConversionStrategy::Transparent(error_mode.clone());

            // DMR: Create a context where proto field is actually optional
            // This would be a case like: Option<TransparentWrapper> -> Option<inner_proto_type>
            let (field, ctx) = test_helpers::create_mock_context(
                "TestStruct",
                "test_field",
                "Option<TransparentWrapper>",
                "proto",
                &["transparent"],
            );

            let rust_field_info = RustFieldInfo::analyze(&ctx, &field);

            // DMR: Manually create proto field info that's optional to trigger the error mode branches
            let proto_field_info = ProtoFieldInfo {
                type_name: "Option<inner_type>".to_string(),
                mapping: ProtoMapping::Scalar,
                optionality: FieldOptionality::Optional, // This makes is_optional() return true
            };

            let proto_to_rust = strategy.generate_proto_to_rust_conversion(
                &ctx,
                &field,
                &rust_field_info,
                &proto_field_info,
            );
            let code_str = proto_to_rust.to_string();
            println!("Generated code for {:?}: {}", error_mode, code_str);

            match &error_mode {
                ErrorMode::Panic => {
                    assert!(code_str.contains("expect"), "Panic mode should use expect");
                }
                ErrorMode::Error => {
                    assert!(
                        code_str.contains("ok_or_else"),
                        "Error mode should use ok_or_else"
                    );
                    assert!(
                        code_str.contains("TestStructConversionError"),
                        "Should use generated error type"
                    );
                }
                ErrorMode::Default(Some(_)) => {
                    assert!(
                        code_str.contains("test_default"),
                        "Should use custom default"
                    );
                }
                ErrorMode::Default(None) => {
                    assert!(
                        code_str.contains("Default :: default"),
                        "Should use Default trait"
                    );
                    assert!(
                        code_str.contains("or_else"),
                        "Should use or_else for Option handling"
                    );
                }
                ErrorMode::None => {
                    assert!(
                        code_str.contains("map"),
                        "None mode should use map for optional"
                    );
                    assert!(
                        code_str.contains("TransparentWrapper :: from"),
                        "Should map with From conversion"
                    );
                }
            }
        }
    }

    #[test]
    fn test_custom_strategy_code_generation() {
        let custom_strategies = vec![
            CustomConversionStrategy::FromFn("custom_from".to_string()),
            CustomConversionStrategy::IntoFn("custom_into".to_string()),
            CustomConversionStrategy::Bidirectional(
                "custom_from".to_string(),
                "custom_into".to_string(),
            ),
        ];

        for custom_strategy in custom_strategies {
            let strategy = FieldConversionStrategy::Custom(custom_strategy.clone());
            let (field, ctx) = test_helpers::create_mock_context(
                "TestStruct",
                "test_field",
                "CustomType",
                "proto",
                &[],
            );
            let rust_field_info = RustFieldInfo::analyze(&ctx, &field);
            let proto_field_info = ProtoFieldInfo::infer_from(&ctx, &field, &rust_field_info);

            let proto_to_rust = strategy.generate_proto_to_rust_conversion(
                &ctx,
                &field,
                &rust_field_info,
                &proto_field_info,
            );
            let rust_to_proto = strategy.generate_rust_to_proto_conversion(
                &ctx,
                &field,
                &rust_field_info,
                &proto_field_info,
            );

            let proto_to_rust_str = proto_to_rust.to_string();
            let rust_to_proto_str = rust_to_proto.to_string();

            match &custom_strategy {
                CustomConversionStrategy::FromFn(fn_name) => {
                    assert!(
                        proto_to_rust_str.contains(fn_name),
                        "Should use custom from function"
                    );
                }
                CustomConversionStrategy::IntoFn(fn_name) => {
                    assert!(
                        rust_to_proto_str.contains(fn_name),
                        "Should use custom into function"
                    );
                }
                CustomConversionStrategy::Bidirectional(from_fn, into_fn) => {
                    assert!(
                        proto_to_rust_str.contains(from_fn),
                        "Should use custom from function"
                    );
                    assert!(
                        rust_to_proto_str.contains(into_fn),
                        "Should use custom into function"
                    );
                }
            }
        }
    }

    #[test]
    fn test_custom_strategy_with_error_code_generation() {
        let custom_strategy = CustomConversionStrategy::Bidirectional(
            "custom_from".to_string(),
            "custom_into".to_string(),
        );

        let error_modes = vec![ErrorMode::Panic, ErrorMode::Error, ErrorMode::Default(None)];

        for error_mode in error_modes {
            let strategy = FieldConversionStrategy::CustomWithError(
                custom_strategy.clone(),
                error_mode.clone(),
            );
            let (field, ctx) = test_helpers::create_mock_context(
                "TestStruct",
                "test_field",
                "CustomComplexType",
                "proto",
                &[
                    "proto_to_rust_fn = \"custom_from\"",
                    "rust_to_proto_fn = \"custom_into\"",
                ],
            );
            let rust_field_info = RustFieldInfo::analyze(&ctx, &field);
            let proto_field_info = ProtoFieldInfo::infer_from(&ctx, &field, &rust_field_info);

            let proto_to_rust = strategy.generate_proto_to_rust_conversion(
                &ctx,
                &field,
                &rust_field_info,
                &proto_field_info,
            );
            let code_str = proto_to_rust.to_string();
            println!("generated code: {}", debug::format_rust_code(&code_str));

            // Should contain the custom function name
            assert!(
                code_str.contains("custom_from"),
                "Should use custom from function"
            );

            // Should have appropriate error handling based on mode
            match &error_mode {
                ErrorMode::Panic => {
                    assert!(code_str.contains("expect"), "Panic mode should use expect");
                }
                ErrorMode::Error => {
                    assert!(
                        code_str.contains("expect"),
                        "Error mode should use expect for custom conversion"
                    );
                    assert!(
                        code_str.contains("custom_from"),
                        "Should use custom from function"
                    );
                }
                ErrorMode::Default(None) => {
                    assert!(
                        code_str.contains("expect"),
                        "Custom conversion uses expect for all error modes"
                    );
                    assert!(
                        code_str.contains("custom_from"),
                        "Should use custom from function"
                    );
                }
                _ => {}
            }
        }
    }
}
