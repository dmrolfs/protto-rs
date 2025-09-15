use quote::quote;
use crate::conversion::ConversionStrategy;
use crate::debug::CallStackDebug;
use crate::field::{
    info::{ProtoFieldInfo, ProtoMapping, RustFieldInfo},
    field_processor::generate_default_value,
};
use crate::validation::ValidationError;
use crate::analysis::{
    attribute_parser,
    expect_analysis::ExpectMode,
    type_analysis,
};
use crate::error_handler;

pub fn generate_field_conversions(
    field: &syn::Field,
    ctx: &FieldProcessingContext,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), ValidationError> {
    let _trace = CallStackDebug::with_context(
        "generate_field_conversions",
        ctx.struct_name,
        ctx.field_name,
        &[],
    );

    let analysis = FieldAnalysis::analyze(ctx, field)?;

    _trace.checkpoint_data(
        "field_analysis",
        &[
            ("category", analysis.conversion_strategy.category()),
            ("debug_info", analysis.conversion_strategy.debug_info()),
        ],
    );
    let proto_to_rust = analysis.generate_proto_to_rust_conversion(ctx);
    let rust_to_proto = analysis.generate_rust_to_proto_conversion(ctx);
    Ok((proto_to_rust, rust_to_proto))
}

#[derive(Debug, Clone)]
pub struct FieldAnalysis {
    pub rust_field: RustFieldInfo,
    pub proto_field: ProtoFieldInfo,
    pub conversion_strategy: ConversionStrategy,
}

impl FieldAnalysis {
    pub fn analyze(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
    ) -> Result<Self, ValidationError> {
        let _trace = CallStackDebug::new("FieldAnalysis", ctx.struct_name, ctx.field_name);

        let rust_field = RustFieldInfo::analyze(ctx, field);
        let proto_field = ProtoFieldInfo::infer_from(ctx, field, &rust_field);
        let conversion_strategy =
            ConversionStrategy::from_field_info(ctx, field, &rust_field, &proto_field);

        if let Err(validation_message) =
            conversion_strategy.validate_for_context(ctx, &rust_field, &proto_field)
        {
            _trace.error(&format!(
                "Invalid conversion strategy for field `{}.{}`: {}",
                ctx.struct_name, ctx.field_name, validation_message,
            ));
            return Err(ValidationError::new(
                ctx,
                &rust_field,
                &proto_field,
                &conversion_strategy,
                validation_message,
            ));
        }

        Ok(Self {
            rust_field,
            proto_field,
            conversion_strategy,
        })
    }

    pub fn generate_proto_to_rust_conversion(
        &self,
        ctx: &FieldProcessingContext,
    ) -> proc_macro2::TokenStream {
        let _trace = CallStackDebug::with_context(
            "generate_proto_to_rust_conversion",
            ctx.struct_name,
            ctx.field_name,
            &[("strategy", &format!("{:?}", self.conversion_strategy))],
        );

        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;

        let result = match &self.conversion_strategy {
            // -- Ignore fields - generate default/function call --
            ConversionStrategy::ProtoIgnore => {
                if let Some(default_fn_name) = &ctx.default_fn {
                    _trace.decision("ProtoIgnore + default_fn", "use custom default function");
                    let default_fn_path: syn::Path = syn::parse_str(default_fn_name)
                        .expect("Failed to parse default_fn function path");
                    quote! { #field_name: #default_fn_path() }
                } else {
                    _trace.decision("ProtoIgnore + no default_fn", "use Default::default");
                    quote! { #field_name: Default::default() }
                }
            }

            // -- Custom derive functions --
            ConversionStrategy::DeriveBidirectional(from_with_path, _) => {
                // Use from_with for proto->rust conversion
                _trace.decision(
                    "DeriveBidirectional_proto_to_rust",
                    &format!("path: {}", from_with_path),
                );
                let from_with_path: syn::Path =
                    syn::parse_str(from_with_path).expect("Failed to parse from_proto_fn path");

                if self.rust_field.is_option && self.proto_field.is_optional() {
                    // Custom function handles Option<T> -> Option<U> transformation
                    quote! { #field_name: #from_with_path(proto_struct.#proto_field_ident) }
                } else if self.proto_field.is_optional() {
                    // Custom function expects unwrapped value - use single line quote
                    quote! { #field_name: #from_with_path(proto_struct.#proto_field_ident.expect(&format!("Proto field {} is required for custom conversion", stringify!(#proto_field_ident)))) }
                } else {
                    quote! { #field_name: #from_with_path(proto_struct.#proto_field_ident) }
                }
            }
            ConversionStrategy::DeriveProtoToRust(from_with_path) => {
                // Handle standalone DeriveFromWith in rust->proto (fallback to from_with
                _trace.decision("DeriveProtoToRust", &format!("path: {}", from_with_path));
                let from_with_path: syn::Path =
                    syn::parse_str(from_with_path).expect("Failed to parse from_proto_fn path");

                if self.rust_field.is_option && self.proto_field.is_optional() {
                    // custom function handles Option<T> -> Option<U> transformation
                    quote! { #field_name: #from_with_path(proto_struct.#proto_field_ident) }
                } else if self.proto_field.is_optional() {
                    // custom fn expects unwrapped value
                    quote! {
                        #field_name: #from_with_path(
                            proto_struct.#proto_field_ident
                                .expect(&format!(
                                    "Proto field {} is required for custom conversion",
                                    stringify!(#proto_field_ident)
                            ))
                        )
                    }
                } else {
                    quote! { #field_name: #from_with_path(proto_struct.#proto_field_ident) }
                }
            }
            ConversionStrategy::DeriveRustToProto(_) => {
                // Handle standalone DeriveIntoWith in proto->rust (fallback to .into())
                _trace.decision("DeriveRustToProto", "fallback to DirectWithInto");
                if self.proto_field.is_optional() {
                    quote! {
                        #field_name: proto_struct.#proto_field_ident
                            .expect(&format!("Proto field {} is required", stringify!(#proto_field_ident)))
                            .into()
                    }
                } else {
                    quote! { #field_name: proto_struct.#proto_field_ident.into() }
                }
            }

            ConversionStrategy::TransparentRequired => {
                _trace.decision(
                    "TransparentRequired",
                    "transparent conversion with option handling",
                );

                let field_type = ctx.field_type;

                // Check if proto field is optional and handle accordingly
                if self.proto_field.is_optional() {
                    quote! {
                        #field_name: #field_type::from(
                            proto_struct.#proto_field_ident
                                .expect(&format!("Proto field {} is required for transparent conversion", stringify!(#proto_field_ident)))
                        )
                    }
                } else {
                    // Direct conversion for required proto fields
                    quote! {
                        #field_name: #field_type::from(proto_struct.#proto_field_ident)
                    }
                }
            }
            ConversionStrategy::TransparentOptionalWithExpect => {
                _trace.decision("TransparentOptionalWithExpect", "expect with panic message");
                let field_type = ctx.field_type;
                quote! {
                    #field_name: #field_type::from(
                        proto_struct.#proto_field_ident
                            .expect(&format!("Proto field {} is required", stringify!(#proto_field_ident)))
                    )
                }
            }
            ConversionStrategy::TransparentOptionalWithError => {
                _trace.decision(
                    "TransparentOptionalWithError",
                    "generate error handling with strategy context",
                );
                error_handler::generate_error_handling(
                    &self.conversion_strategy,
                    field_name,
                    proto_field_ident,
                    ctx.field_type,
                    &ctx.proto_meta,
                    ctx.error_name,
                    ctx.struct_level_error_type,
                    ctx.struct_level_error_fn,
                )
            }
            ConversionStrategy::TransparentOptionalWithDefault => {
                _trace.decision(
                    "TransparentOptionalWithDefault",
                    "unwrap_or_else with default",
                );
                let field_type = ctx.field_type;
                let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
                quote! {
                    #field_name: #field_type::from(
                        proto_struct.#proto_field_ident.unwrap_or_else(|| {
                            // Convert default to proto type first, then to rust type
                            let default_val: #field_type = #default_expr;
                            default_val.into()
                        })
                    )
                }
            }

            // -- Direct conversions --
            ConversionStrategy::DirectAssignment => {
                _trace.decision("DirectAssignment", "direct assignment");
                quote! { #field_name: proto_struct.#proto_field_ident }
            }
            ConversionStrategy::DirectWithInto => {
                _trace.decision("DirectWithInto", "direct with into");
                quote! { #field_name: proto_struct.#proto_field_ident.into() }
            }

            // -- Option handling --
            ConversionStrategy::WrapInSome => {
                _trace.decision("WrapInSome", "wrap proto value in Some");
                quote! { #field_name: Some(proto_struct.#proto_field_ident.into()) }
            }
            ConversionStrategy::UnwrapOptionalWithExpect => {
                _trace.decision("UnwrapOptionalWithExpect", "expect with panic message");
                quote! {
                    #field_name: proto_struct.#proto_field_ident
                        .expect(&format!("Proto field {} is required", stringify!(#proto_field_ident)))
                        .into()
                }
            }
            ConversionStrategy::UnwrapOptionalWithError => {
                _trace.decision(
                    "UnwrapOptionalWithError",
                    "generate error handling with strategy context",
                );
                error_handler::generate_error_handling(
                    &self.conversion_strategy,
                    field_name,
                    proto_field_ident,
                    ctx.field_type,
                    &ctx.proto_meta,
                    ctx.error_name,
                    ctx.struct_level_error_type,
                    ctx.struct_level_error_fn,
                )
            }
            ConversionStrategy::UnwrapOptionalWithDefault => {
                _trace.decision("UnwrapOptionalWithDefault", "unwrap_or_else with default");
                let default_expr =
                    generate_default_value(ctx.field_type, ctx.default_fn.as_deref());
                quote! {
                    #field_name: proto_struct.#proto_field_ident
                        .map(|v| v.into())
                        .unwrap_or_else(|| #default_expr)
                }
            }
            ConversionStrategy::MapOption => {
                _trace.decision("MapOption", "map option with transparent handling");

                // Check if this is a transparent option case
                if self.rust_field.has_transparent && self.rust_field.is_option {
                    if let Some(inner_type) = self.rust_field.get_inner_type() {
                        quote! {
                            #field_name: proto_struct.#proto_field_ident.map(|proto_val| {
                                #inner_type::from(proto_val)
                            })
                        }
                    } else {
                        quote! { #field_name: proto_struct.#proto_field_ident.map(|v| v.into()) }
                    }
                } else {
                    // Normal option mapping
                    quote! { #field_name: proto_struct.#proto_field_ident.map(|v| v.into()) }
                }
            }
            ConversionStrategy::MapOptionWithDefault => {
                _trace.decision("MapOptionWithDefault", "map option with default fallback");
                let default_expr =
                    generate_default_value(ctx.field_type, ctx.default_fn.as_deref());
                quote! {
                    #field_name: proto_struct.#proto_field_ident
                        .map(|v| v.into())
                        .or_else(|| #default_expr)
                }
            }

            // -- Collection handling --
            ConversionStrategy::CollectVec => {
                _trace.decision("CollectVec", "collect with into_iter.map");
                if self.rust_field.is_option {
                    // Proto Vec<T> → Rust Option<Vec<U>> - preserve None for empty vecs
                    quote! {
                        #field_name: if proto_struct.#proto_field_ident.is_empty() {
                            None
                        } else {
                            Some(proto_struct.#proto_field_ident.into_iter().map(Into::into).collect())
                        }
                    }
                } else {
                    // Proto Vec<T> → Rust Vec<U>
                    quote! {
                        #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                    }
                }
            }
            ConversionStrategy::CollectVecWithDefault => {
                _trace.decision(
                    "CollectVecWithDefault",
                    "check empty then collect or default",
                );
                let default_expr =
                    generate_default_value(ctx.field_type, ctx.default_fn.as_deref());
                quote! {
                    #field_name: {
                        if proto_struct.#proto_field_ident.is_empty() {
                            #default_expr
                        } else {
                            proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                        }
                    }
                }
            }
            ConversionStrategy::CollectVecWithError if ctx.default_fn.is_some() => {
                _trace.decision(
                    "CollectVecWithError + has_default",
                    "use default when empty, error on conversion failures",
                );
                let default_expr =
                    generate_default_value(ctx.field_type, ctx.default_fn.as_deref());
                quote! {
                    #field_name: if proto_struct.#proto_field_ident.is_empty() {
                        #default_expr
                    } else {
                        proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                    }
                }
            }
            ConversionStrategy::CollectVecWithError => {
                _trace.decision(
                    "CollectVecWithError + no_default",
                    "generate error handling for vec with strategy context",
                );
                error_handler::generate_error_handling(
                    &self.conversion_strategy,
                    field_name,
                    proto_field_ident,
                    ctx.field_type,
                    &ctx.proto_meta,
                    ctx.error_name,
                    ctx.struct_level_error_type,
                    ctx.struct_level_error_fn,
                )
            }
            ConversionStrategy::VecDirectAssignment => {
                _trace.decision(
                    "VecDirectAssignment",
                    "direct assignment for proto vec types",
                );
                quote! { #field_name: proto_struct.#proto_field_ident }
            }
            ConversionStrategy::MapVecInOption => {
                _trace.decision("MapVecInOption", "map option vec with collect");
                let inner_type = type_analysis::get_inner_type_from_option(ctx.field_type)
                    .and_then(|t| type_analysis::get_inner_type_from_vec(&t));

                if let Some(inner) = inner_type {
                    if type_analysis::is_proto_type(&inner, ctx.proto_module) {
                        // Option<Vec<ProtoType>> -> direct map without conversion
                        quote! { #field_name: proto_struct.#proto_field_ident.map(|vec| vec) }
                    } else {
                        // Option<Vec<CustomType>> -> map with conversion
                        quote! {
                            #field_name: proto_struct.#proto_field_ident
                                .map(|vec| vec.into_iter().map(Into::into).collect())
                        }
                    }
                } else {
                    quote! {
                        #field_name: proto_struct.#proto_field_ident
                            .map(|vec| vec.into_iter().map(Into::into).collect())
                    }
                }
            }

            // -- Rust-to-proto specific strategies (handled in other direction) --
            ConversionStrategy::UnwrapOptional => {
                _trace.decision("UnwrapOptional_fallback", "falling back to DirectWithInto");
                quote! { #field_name: proto_struct.#proto_field_ident.into() }
            }

            // -- Fallback for complex cases --
            ConversionStrategy::RequiresCustomLogic => {
                _trace.error("RequiresCustomLogic strategy should not reach code generation");
                panic!(
                    "Custom logic required for field '{}' - this should be handled separately",
                    field_name
                );
            }
        };

        _trace.generated_code(
            &result,
            ctx.struct_name,
            ctx.field_name,
            "proto_to_rust",
            &[],
        );

        result
    }

    pub fn generate_rust_to_proto_conversion(
        &self,
        ctx: &FieldProcessingContext,
    ) -> proc_macro2::TokenStream {
        let _trace = CallStackDebug::with_context(
            "generate_rust_to_proto_conversion",
            ctx.struct_name,
            ctx.field_name,
            &[("strategy", &format!("{:?}", self.conversion_strategy))],
        );

        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;

        let result = match &self.conversion_strategy {
            // -- Ignore fields - not included in proto struct --
            ConversionStrategy::ProtoIgnore => {
                _trace.decision("ProtoIgnore", "return empty - field not in proto");
                proc_macro2::TokenStream::new()
            }

            // -- Custom derive functions --
            ConversionStrategy::DeriveBidirectional(_, into_with_path) => {
                // Use into_with for rust->proto conversion
                _trace.decision(
                    "DeriveBidirectional_rust_to_proto",
                    &format!("path: {}", into_with_path),
                );
                let into_with_path: syn::Path =
                    syn::parse_str(into_with_path).expect("Failed to parse to_proto_fn path");

                if self.rust_field.is_option && self.proto_field.is_optional() {
                    quote! { #proto_field_ident: #into_with_path(my_struct.#field_name) }
                } else if self.proto_field.is_optional() {
                    quote! { #proto_field_ident: Some(#into_with_path(my_struct.#field_name)) }
                } else {
                    quote! { #proto_field_ident: #into_with_path(my_struct.#field_name) }
                }
            }
            ConversionStrategy::DeriveProtoToRust(_) => {
                // Handle standalone DeriveFromWith in rust->proto (fallback to .into())
                _trace.decision("DeriveProtoToRust", "fallback to DirectWithInto");
                if self.rust_field.is_option && self.proto_field.is_optional() {
                    quote! { #proto_field_ident: my_struct.#field_name }
                } else if self.proto_field.is_optional() {
                    quote! { #proto_field_ident: Some(my_struct.#field_name.into()) }
                } else {
                    quote! { #proto_field_ident: my_struct.#field_name.into() }
                }
            }
            ConversionStrategy::DeriveRustToProto(into_with_path) => {
                _trace.decision("DeriveRustToProto", &format!("path: {}", into_with_path));
                let into_with_path: syn::Path =
                    syn::parse_str(into_with_path).expect("Failed to parse to_proto_fn path");

                if self.rust_field.is_option && self.proto_field.is_optional() {
                    quote! { #proto_field_ident: #into_with_path(my_struct.#field_name) }
                } else if self.proto_field.is_optional() {
                    quote! { #proto_field_ident: Some(#into_with_path(my_struct.#field_name)) }
                } else {
                    quote! { #proto_field_ident: #into_with_path(my_struct.#field_name) }
                }
            }

            // -- Direct conversions --
            ConversionStrategy::DirectAssignment => {
                _trace.decision("DirectAssignment", "direct assignment");
                quote! { #proto_field_ident: my_struct.#field_name }
            }
            ConversionStrategy::DirectWithInto => {
                _trace.decision("DirectWithInto", "direct with into");
                quote! { #proto_field_ident: my_struct.#field_name.into() }
            }

            // -- Option handling --
            ConversionStrategy::WrapInSome => {
                _trace.decision("WrapInSome", "wrap in Some with into");
                quote! { #proto_field_ident: Some(my_struct.#field_name.into()) }
            }
            ConversionStrategy::UnwrapOptional if self.proto_field.is_optional() => {
                _trace.decision(
                    "UnwrapOptional + proto_optional",
                    "map instead of unwrap for Option->Option",
                );
                quote! { #proto_field_ident: my_struct.#field_name.map(|v| v.into()) }
            }
            ConversionStrategy::UnwrapOptional => {
                _trace.decision(
                    "UnwrapOptional + proto_required",
                    "unwrap with unwrap_or_default",
                );
                quote! { #proto_field_ident: my_struct.#field_name.unwrap_or_default().into() }
            }
            ConversionStrategy::MapOption => {
                _trace.decision("MapOption", "map option rust->proto");
                quote! { #proto_field_ident: my_struct.#field_name.map(|v| v.into()) }
            }
            ConversionStrategy::MapOptionWithDefault => {
                _trace.decision("MapOptionWithDefault", "map option with into");
                quote! { #proto_field_ident: my_struct.#field_name.map(|v| v.into()) }
            }

            // -- Collection handling --
            ConversionStrategy::CollectVec => {
                _trace.decision("CollectVec", "collect with into_iter.map");
                if self.rust_field.is_option {
                    // Rust Option<Vec<T>> → Proto Vec<U>
                    quote! {
                        #proto_field_ident: my_struct.#field_name.unwrap_or_default().into_iter().map(Into::into).collect()
                    }
                } else {
                    // Rust Vec<T> → Proto Vec<U>
                    quote! {
                        #proto_field_ident: my_struct.#field_name.into_iter().map(Into::into).collect()
                    }
                }
            }
            ConversionStrategy::MapVecInOption => {
                _trace.decision("MapVecInOption", "map option vec with collect");
                quote! {
                    #proto_field_ident: my_struct.#field_name
                        .map(|vec| vec.into_iter().map(Into::into).collect())
                }
            }
            ConversionStrategy::VecDirectAssignment => {
                _trace.decision("VecDirectAssignment", "direct assignment for proto types");
                quote! { #proto_field_ident: my_struct.#field_name }
            }

            // -- Proto-to-rust specific strategies - fall back to appropriate rust-to-proto logic --
            ConversionStrategy::UnwrapOptionalWithExpect => {
                _trace.decision(
                    "UnwrapOptionalWithExpect",
                    "wrap in Some for proto optional",
                );
                quote! { #proto_field_ident: Some(my_struct.#field_name.into()) }
            }

            ConversionStrategy::UnwrapOptionalWithError
            | ConversionStrategy::UnwrapOptionalWithDefault => {
                // These are proto-to-rust specific, fall back to appropriate rust-to-proto logic
                _trace.decision(
                    "proto_to_rust_specific_strategy",
                    "determine appropriate rust_to_proto conversion",
                );

                if self.rust_field.is_option {
                    quote! { #proto_field_ident: my_struct.#field_name.map(|v| v.into()) }
                } else if self.proto_field.is_optional() {
                    quote! { #proto_field_ident: Some(my_struct.#field_name.into()) }
                } else {
                    quote! { #proto_field_ident: my_struct.#field_name.into() }
                }
            }

            ConversionStrategy::TransparentRequired => {
                _trace.decision(
                    "TransparentRequired",
                    "transparent rust->proto using Into trait",
                );

                // Check if proto field is optional and wrap accordingly
                if self.proto_field.is_optional() {
                    quote! { #proto_field_ident: Some(my_struct.#field_name.into()) }
                } else {
                    // Direct conversion for required proto fields
                    quote! { #proto_field_ident: my_struct.#field_name.into() }
                }
            }

            ConversionStrategy::TransparentOptionalWithExpect
            | ConversionStrategy::TransparentOptionalWithError
            | ConversionStrategy::TransparentOptionalWithDefault => {
                _trace.decision("transparent_rust_to_proto", "use Into conversion");
                quote! { #proto_field_ident: Some(my_struct.#field_name.into()) }
            }

            ConversionStrategy::CollectVecWithDefault | ConversionStrategy::CollectVecWithError => {
                // These are proto-to-rust specific, fall back to appropriate rust-to-proto logic
                _trace.decision(
                    "proto_to_rust_specific_strategy",
                    "determine appropriate rust_to_proto conversion",
                );

                let rust = &self.rust_field;
                let proto = &self.proto_field;

                match (rust.is_option, proto.mapping) {
                    (true, ProtoMapping::Optional) => {
                        quote! { #proto_field_ident: my_struct.#field_name.map(|v| v.into()) }
                    }

                    (true, ProtoMapping::Repeated) => {
                        // Option<Vec<T>> -> Vec<T> - unwrap with default empty vec
                        _trace.decision(
                            "option_vec_to_vec_conversion",
                            "unwrap_or_default then collect",
                        );
                        quote! { #proto_field_ident: my_struct.#field_name.unwrap_or_default().into_iter().map(Into::into).collect() }
                    }

                    (true, ProtoMapping::Scalar | ProtoMapping::Message) => quote! {
                        #proto_field_ident: my_struct.#field_name.unwrap_or_default().into()
                    },

                    (true, ProtoMapping::CustomDerived) => {
                        // Option<CustomType> -> CustomType - unwrap with default
                        _trace.decision(
                            "option_custom_derived_fallback",
                            "unwrap_or_default then into",
                        );
                        quote! { #proto_field_ident: my_struct.#field_name.unwrap_or_default().into() }
                    }

                    (false, ProtoMapping::Optional) => quote! {
                        #proto_field_ident: Some(my_struct.#field_name.into())
                    },

                    (false, ProtoMapping::Repeated) => {
                        _trace.decision("vec_to_vec_conversion", "collect with into_iter.map");
                        quote! {
                            #proto_field_ident: my_struct.#field_name
                                .into_iter()
                                .map(Into::into)
                                .collect()
                        }
                    }

                    (false, ProtoMapping::Scalar | ProtoMapping::Message) => {
                        _trace.decision("scalar_to_scalar_conversion", "direct into conversion");
                        quote! { #proto_field_ident: my_struct.#field_name.into() }
                    }

                    (false, ProtoMapping::CustomDerived) => {
                        _trace.decision("custom_derived_fallback", "direct into conversion");
                        quote! { #proto_field_ident: my_struct.#field_name.into() }
                    }
                }
            }

            // -- Proto-to-rust only strategies --
            ConversionStrategy::RequiresCustomLogic => {
                _trace.error("RequiresCustomLogic strategy should not reach code generation");
                panic!(
                    "Custom logic required for field '{}' - this should be handled separately",
                    field_name
                );
            }
        };

        _trace.generated_code(
            &result,
            ctx.struct_name,
            ctx.field_name,
            "rust_to_proto",
            &[],
        );

        result
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
            struct_level_error_type,
            struct_level_error_fn,
            proto_module,
            proto_name,
        }
    }
}
