use crate::debug::CallStackDebug;
use super::*;
use crate::expect_analysis::ExpectMode;
use crate::field_processor::generate_default_value;
use crate::optionality::FieldOptionality;

pub fn generate_field_conversions(
    field: &syn::Field,
    ctx: &FieldProcessingContext
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let _trace = CallStackDebug::with_context(
        "generate_field_conversions",
        ctx.struct_name,
        ctx.field_name,
        &[],
    );

    let analysis = FieldAnalysis::analyze(ctx, field);
    _trace.checkpoint_data(
        "field_analysis",
        &[
            ("category", analysis.conversion_strategy.category()),
            ("debug_info", analysis.conversion_strategy.debug_info()),
        ]
    );
    let proto_to_rust = analysis.generate_proto_to_rust_conversion(ctx);
    let rust_to_proto = analysis.generate_rust_to_proto_conversion(ctx);
    (proto_to_rust, rust_to_proto)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldAnalysis {
    pub rust_field: RustFieldInfo,
    pub proto_field: ProtoFieldInfo,
    pub conversion_strategy: ConversionStrategy,
}

impl FieldAnalysis {
    pub fn analyze(ctx: &FieldProcessingContext, field: &syn::Field) -> Self {
        let rust_field = RustFieldInfo::analyze(ctx, field);
        let proto_field = ProtoFieldInfo::infer_from(ctx, field, &rust_field);
        let conversion_strategy = ConversionStrategy::from_field_info(
            ctx,
            field,
            &rust_field,
            &proto_field
        );

        if let Err(validation_error) = conversion_strategy.validate_for_context(ctx, &rust_field, &proto_field) {
            panic!("Invalid conversion strategy for field '{}.{}': {}",
                   ctx.struct_name, ctx.field_name, validation_error);
        }

        Self {
            rust_field,
            proto_field,
            conversion_strategy,
        }
    }

    pub fn generate_proto_to_rust_conversion(
        &self,
        ctx: &FieldProcessingContext
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
                    let default_fn_path: syn::Path = syn::parse_str(&default_fn_name)
                        .expect("Failed to parse default_fn function path");
                    quote! { #field_name: #default_fn_path() }
                } else {
                    _trace.decision("ProtoIgnore + no default_fn", "use Default::default");
                    quote! { #field_name: Default::default() }
                }
            },

            // -- Custom derive functions --
            ConversionStrategy::DeriveFromWith(from_with_path) => {
                // Handle standalone DeriveFromWith in rust->proto (fallback to from_with
                _trace.decision("DeriveFromWith", &format!("path: {}", from_with_path));
                let from_with_path: syn::Path = syn::parse_str(&from_with_path)
                    .expect("Failed to parse derive_from_with path");
                quote! { #field_name: #from_with_path(proto_struct.#proto_field_ident) }
            },
            ConversionStrategy::DeriveBidirectional(from_with_path, _) => {
                // Use from_with for proto->rust conversion
                _trace.decision("DeriveBidirectional_proto_to_rust", &format!("path: {}", from_with_path));
                let from_with_path: syn::Path = syn::parse_str(&from_with_path)
                    .expect("Failed to parse derive_from_with path");
                quote! { #field_name: #from_with_path(proto_struct.#proto_field_ident) }
            },

            ConversionStrategy::DeriveIntoWith(_) => {
                // Handle standalone DeriveIntoWith in proto->rust (fallback to .into())
                _trace.decision("DeriveIntoWith_in_proto_to_rust", "fallback to DirectWithInto");
                quote! { #field_name: proto_struct.#proto_field_ident.into() }
            },

            // -- Transparent field handling --
            ConversionStrategy::TransparentRequired => {
                _trace.decision("TransparentRequired", "direct from conversion");
                let field_type = ctx.field_type;
                quote! { #field_name: <#field_type>::from(proto_struct.#proto_field_ident) }
            },
            ConversionStrategy::TransparentOptionalWithExpect => {
                _trace.decision("TransparentOptionalWithExpect", "expect with panic message");
                let field_type = ctx.field_type;
                quote! {
                    #field_name: <#field_type>::from(
                        proto_struct.#proto_field_ident
                            .expect(&format!("Proto field {} is required", stringify!(#proto_field_ident)))
                    )
                }
            },
            ConversionStrategy::TransparentOptionalWithError => {
                _trace.decision("TransparentOptionalWithError", "generate error handling with strategy context");
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
            },
            ConversionStrategy::TransparentOptionalWithDefault => {
                _trace.decision("TransparentOptionalWithDefault", "unwrap_or_else with default");
                let field_type = ctx.field_type;
                let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
                quote! {
                    #field_name: <#field_type>::from(
                        proto_struct.#proto_field_ident
                            .unwrap_or_else(|| #default_expr)
                    )
                }
            },

            // -- Direct conversions --
            ConversionStrategy::DirectAssignment => {
                _trace.decision("DirectAssignment", "direct assignment");
                quote! { #field_name: proto_struct.#proto_field_ident }
            },
            ConversionStrategy::DirectWithInto => {
                _trace.decision("DirectWithInto", "direct with into");
                quote! { #field_name: proto_struct.#proto_field_ident.into() }
            },

            // -- Option handling --
            ConversionStrategy::WrapInSome => {
                _trace.decision("WrapInSome", "wrap proto value in Some");
                quote! { #field_name: Some(proto_struct.#proto_field_ident.into()) }
            },
            // ConversionStrategy::UnwrapOptional => {
            //     todo: Is this correct?
                // _trace.decision("UnwrapOptional", "unwrap with unwrap_or_default");
                // quote! { #field_name: proto_struct.#proto_field_ident.unwrap_or_default().into() }
            // },
            ConversionStrategy::UnwrapOptionalWithExpect => {
                _trace.decision("UnwrapOptionalWithExpect", "expect with panic message");
                quote! {
                    #field_name: proto_struct.#proto_field_ident
                        .expect(&format!("Proto field {} is required", stringify!(#proto_field_ident)))
                        .into()
                }
            },
            ConversionStrategy::UnwrapOptionalWithError => {
                _trace.decision("UnwrapOptionalWithError", "generate error handling with strategy context");
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
            },
            ConversionStrategy::UnwrapOptionalWithDefault => {
                _trace.decision("UnwrapOptionalWithDefault", "unwrap_or_else with default");
                let default_expr = generate_default_value(ctx.field_type, ctx.default_fn.as_deref());
                quote! {
                    #field_name: proto_struct.#proto_field_ident
                        .map(|v| v.into())
                        .unwrap_or_else(|| #default_expr)
                }
            },
            ConversionStrategy::MapOption => {
                _trace.decision("MapOption", "map with into");
                quote! { #field_name: proto_struct.#proto_field_ident.map(|v| v.into()) }
            },
            ConversionStrategy::MapOptionWithDefault => {
                _trace.decision("MapOptionWithDefault", "map option with default fallback");
                let default_expr = generate_default_value(ctx.field_type, ctx.default_fn.as_deref());
                quote! {
                    #field_name: proto_struct.#proto_field_ident
                        .map(|v| v.into())
                        .or_else(|| #default_expr)
                }
            },

            // -- Collection handling --
            ConversionStrategy::CollectVec => {
                _trace.decision("CollectVec", "collect with into_iter.map");
                quote! {
                    #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                }
            },
            ConversionStrategy::CollectVecWithDefault => {
                _trace.decision("CollectVecWithDefault", "check empty then collect or default");
                let default_expr = generate_default_value(ctx.field_type, ctx.default_fn.as_deref());
                quote! {
                    #field_name: if proto_struct.#proto_field_ident.is_empty() {
                        #default_expr
                    } else {
                        proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                    }
                }
            },
            ConversionStrategy::CollectVecWithError => {
                _trace.decision("CollectVecWithError", "generate error handling for vec with strategy context");
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
            },
            ConversionStrategy::VecDirectAssignment => {
                _trace.decision("VecDirectAssignment", "direct assignment for proto vec types");
                quote! { #field_name: proto_struct.#proto_field_ident }
            },
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
            },

            // -- Rust-to-proto specific strategies (handled in other direction) --
            ConversionStrategy::DeriveIntoWith(_) |
            ConversionStrategy::TransparentToOptional |
            ConversionStrategy::TransparentToRequired |
            ConversionStrategy::UnwrapOptional => {
                _trace.decision("rust_to_proto_strategy_in_proto_to_rust", "fallback to DirectWithInto");
                quote! { #field_name: proto_struct.#proto_field_ident.into() }
            },

            // -- Fallback for complex cases --
            ConversionStrategy::RequiresCustomLogic => {
                _trace.error("RequiresCustomLogic strategy should not reach code generation");
                panic!("Custom logic required for field '{}' - this should be handled separately", field_name);
            }
        };

        _trace.generated_code(
            &result,
            ctx.struct_name,
            ctx.field_name,
            "proto_to_rust",
            &[("strategy", &format!("{:?}", self.conversion_strategy))],
        );

        result
    }

    pub fn generate_rust_to_proto_conversion(
        &self,
        ctx: &FieldProcessingContext
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
                quote! {}
            },

            // -- Custom derive functions --
            ConversionStrategy::DeriveIntoWith(into_with_path) => {
                _trace.decision("DeriveIntoWith", &format!("path: {}", into_with_path));
                let into_with_path: syn::Path = syn::parse_str(&into_with_path)
                    .expect("Failed to parse derive_into_with path");
                quote! { #proto_field_ident: #into_with_path(my_struct.#field_name) }
            },

            ConversionStrategy::DeriveBidirectional(_, into_with_path) => {
                // Use into_with for rust->proto conversion
                _trace.decision("DeriveBidirectional_rust_to_proto", &format!("path: {}", into_with_path));
                let into_with_path: syn::Path = syn::parse_str(&into_with_path)
                    .expect("Failed to parse derive_into_with path");
                quote! { #proto_field_ident: #into_with_path(my_struct.#field_name) }
            },

            ConversionStrategy::DeriveFromWith(_) => {
                // Handle standalone DeriveFromWith in rust->proto (fallback to .into())
                _trace.decision("DeriveFromWith_in_rust_to_proto", "fallback to DirectWithInto");
                quote! { #proto_field_ident: my_struct.#field_name.into() }
            },

            // -- Transparent field handling --
            ConversionStrategy::TransparentToOptional => {
                _trace.decision("TransparentToOptional", "wrap in Some with into");
                quote! { #proto_field_ident: Some(my_struct.#field_name.into()) }
            },
            ConversionStrategy::TransparentToRequired => {
                _trace.decision("TransparentToRequired", "direct into conversion");
                quote! { #proto_field_ident: my_struct.#field_name.into() }
            },

            // -- Direct conversions --
            ConversionStrategy::DirectAssignment => {
                _trace.decision("DirectAssignment", "direct assignment");
                quote! { #proto_field_ident: my_struct.#field_name }
            },
            ConversionStrategy::DirectWithInto => {
                _trace.decision("DirectWithInto", "direct with into");
                quote! { #proto_field_ident: my_struct.#field_name.into() }
            },

            // -- Option handling --
            ConversionStrategy::WrapInSome => {
                _trace.decision("WrapInSome", "wrap in Some with into");
                quote! { #proto_field_ident: Some(my_struct.#field_name.into()) }
            },
            ConversionStrategy::UnwrapOptional => {
                _trace.decision("UnwrapOptional", "unwrap with unwrap_or_default");
                quote! { #proto_field_ident: my_struct.#field_name.unwrap_or_default().into() }
            },
            ConversionStrategy::MapOption => {
                _trace.decision("MapOption", "map with into");
                quote! { #proto_field_ident: my_struct.#field_name.map(|v| v.into()) }
            },
            ConversionStrategy::MapOptionWithDefault => {
                _trace.decision("MapOptionWithDefault", "map option with into");
                quote! { #proto_field_ident: my_struct.#field_name.map(|v| v.into()) }
            },

            // -- Collection handling --
            ConversionStrategy::CollectVec => {
                _trace.decision("CollectVec", "collect with into_iter.map");
                quote! {
                    #proto_field_ident: my_struct.#field_name.into_iter().map(Into::into).collect()
                }
            },
            ConversionStrategy::MapVecInOption => {
                _trace.decision("MapVecInOption", "map option vec with collect");
                quote! {
                    #proto_field_ident: my_struct.#field_name
                        .map(|vec| vec.into_iter().map(Into::into).collect())
                }
            },
            ConversionStrategy::VecDirectAssignment => {
                _trace.decision("VecDirectAssignment", "direct assignment for proto types");
                quote! { #proto_field_ident: my_struct.#field_name }
            },

            // -- Proto-to-rust specific strategies - fall back to appropriate rust-to-proto logic --
            ConversionStrategy::UnwrapOptionalWithExpect |
            ConversionStrategy::UnwrapOptionalWithError |
            ConversionStrategy::UnwrapOptionalWithDefault |
            ConversionStrategy::TransparentRequired |
            ConversionStrategy::TransparentOptionalWithExpect |
            ConversionStrategy::TransparentOptionalWithError |
            ConversionStrategy::TransparentOptionalWithDefault |
            ConversionStrategy::CollectVecWithDefault |
            ConversionStrategy::CollectVecWithError => {
                // These are proto-to-rust specific, fall back to appropriate rust-to-proto logic
                _trace.decision("proto_to_rust_specific_strategy", "determine appropriate rust_to_proto conversion");

                let rust = &self.rust_field;
                let proto = &self.proto_field;

                match (rust.is_option, proto.is_optional()) {
                    (true, true) => quote! {
                        #proto_field_ident: my_struct.#field_name.map(|v| v.into())
                    },
                    (true, false) => quote! {
                        #proto_field_ident: my_struct.#field_name.unwrap_or_default().into()
                    },
                    (false, true) => quote! {
                        #proto_field_ident: Some(my_struct.#field_name.into())
                    },
                    (false, false) => {
                        if rust.is_vec {
                            _trace.decision("vec_to_vec_conversion", "collect with into_iter.map");
                            quote! {
                            #proto_field_ident: my_struct.#field_name
                                .into_iter()
                                .map(Into::into)
                                .collect()
                        }
                        } else {
                            // For non-vec types, use simple .into() conversion
                            _trace.decision("scalar_to_scalar_conversion", "direct into conversion");
                            quote! {
                               #proto_field_ident: my_struct.#field_name.into()
                            }
                        }
                    },
                }
            },

            // -- Proto-to-rust only strategies --
            ConversionStrategy::DeriveFromWith(_) => {
                _trace.decision("DeriveFromWith_in_rust_to_proto", "fallback to DirectWithInto");
                quote! { #proto_field_ident: my_struct.#field_name.into() }
            },

            ConversionStrategy::RequiresCustomLogic => {
                _trace.error("RequiresCustomLogic strategy should not reach code generation");
                panic!("Custom logic required for field '{}' - this should be handled separately", field_name);
            }
        };

        _trace.generated_code(
            &result,
            ctx.struct_name,
            ctx.field_name,
            "rust_to_proto",
            &[("strategy", &format!("{:?}", self.conversion_strategy))],
        );

        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RustFieldInfo {
    pub type_name: String,
    pub is_option: bool,
    pub is_vec: bool,
    pub is_primitive: bool,
    pub is_custom: bool,
    pub is_enum: bool,
    pub has_transparent: bool,
    pub has_default: bool,
    pub expect_mode: ExpectMode,
    pub has_proto_ignore: bool,
    pub derive_from_with: Option<String>,
    pub derive_into_with: Option<String>,
}

impl RustFieldInfo {
    pub fn analyze(ctx: &FieldProcessingContext, field: &syn::Field) -> Self {
        let field_type = ctx.field_type;

        Self {
            type_name: quote!(#field_type).to_string(),
            is_option: type_analysis::is_option_type(field_type),
            is_vec: type_analysis::is_vec_type(field_type),
            is_primitive: type_analysis::is_primitive_type(field_type),
            is_custom: type_analysis::is_custom_type(field_type),
            is_enum: type_analysis::is_enum_type(field_type),
            has_transparent: attribute_parser::has_transparent_attr(field),
            has_default: ctx.has_default,
            expect_mode: ctx.expect_mode,
            has_proto_ignore: attribute_parser::has_proto_ignore(field),
            derive_from_with: attribute_parser::get_proto_derive_from_with(field),
            derive_into_with: attribute_parser::get_proto_derive_into_with(field),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProtoFieldInfo {
    pub type_name: String,
    pub is_vec: bool,
    pub is_primitive: bool,
    pub is_custom: bool,
    pub is_enum: bool,
    pub optionality: FieldOptionality,
}

impl ProtoFieldInfo {
    #[inline]
    pub fn is_optional(&self) -> bool {
        self.optionality.is_optional()
    }
}

impl ProtoFieldInfo {
    pub fn infer_from(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        rust_field: &RustFieldInfo
    ) -> Self {
        let _trace = CallStackDebug::with_context(
            "ProtoFieldInfo::infer_from",
            ctx.struct_name,
            ctx.field_name,
            &[
                ("is_rust_vec", &rust_field.is_vec.to_string()),
                ("is_rust_primitive", &rust_field.is_primitive.to_string()),
                ("is_rust_custom", &rust_field.is_custom.to_string()),
                ("is_rust_enum", &rust_field.is_enum.to_string()),
            ]
        );

        // 1. Try to get explicit optionality from user annotations first
        let optionality = ctx.proto_meta
            .get_proto_optionality()
            .map(|explicit_optionality| {
                _trace.decision("explicit_proto_optionality", "using user annotation");
                *explicit_optionality
            })
            .unwrap_or_else(|| Self::determine_optionality_from_actual_type(
                ctx,
                field,
                rust_field,
                &_trace
            ));

        _trace.checkpoint_data(
            "proto_field_determined",
            &[("optionality", &format!("{:?}", optionality))]
        );

        Self {
            type_name: Self::infer_proto_type_name(ctx, rust_field),
            is_vec: rust_field.is_vec,
            is_primitive: rust_field.is_primitive,
            is_custom: rust_field.is_custom,
            is_enum: rust_field.is_enum,
            optionality,
        }
    }

    /// Determine proto optionality by analyzing what the proto field type actually is
    fn determine_optionality_from_actual_type(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> FieldOptionality {
        // Strategy 1: Use proto definition analysis (if available via build-time metadata)
        Self::try_build_time_proto_analysis(ctx, trace)
            .or_else(|| {
                // Strategy 2: Pattern-based inference from the proto definition context
                Self::infer_from_proto_patterns(ctx, field, rust_field, trace)
            })
            .unwrap_or_else(|| {
                // Strategy 3: Fallback to existing logic but with better detection
                trace.checkpoint("Falling back to existing optionality detection");
                FieldOptionality::from_field_context(ctx, field)
            })
    }

    /// Try to determine optionality from build-time proto analysis
    fn try_build_time_proto_analysis(
        ctx: &FieldProcessingContext,
        trace: &CallStackDebug,
    ) -> Option<FieldOptionality> {
        // This is where you could integrate with prost-build or proto file analysis
        // For now, we'll implement pattern-based detection below
        // trace.checkpoint("build_time_proto_analysis not yet implemented");
        None
    }

    /// Infer proto optionality from patterns and context
    fn infer_from_proto_patterns(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> Option<FieldOptionality> {
        if Self::has_strong_optional_indicators(ctx, field, rust_field, trace) {
            // Pattern 1: If Rust field has explicit optional usage indicators, proto is likely optional
            trace.decision("strong_optional_indicators", "proto field is likely optional");
            Some(FieldOptionality::Optional)
        } else if Self::is_custom_type_likely_optional_in_proto(ctx, field, rust_field, trace) {
            // Pattern 2: Custom types without Option<> in Rust but with expect/error handling
            // often map to optional proto fields that need unwrapping
            trace.decision("custom_type_likely_optional_proto", "proto field is likely optional based on type pattern");
            Some(FieldOptionality::Optional)
        } else if rust_field.is_primitive && !Self::has_any_optional_indicators(ctx, field) {
            // Pattern 3: Primitive types without explicit indicators are often required
            trace.decision("primitive_no_indicators", "proto field likely required");
            Some(FieldOptionality::Required)
        } else {
            trace.checkpoint("no clear proto pattern detected");
            None
        }
    }

    /// Critical pattern for your case - detect custom types that are likely optional in proto
    fn is_custom_type_likely_optional_in_proto(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> bool {
        if !rust_field.is_custom {
            return false;
        }

        // proto enums are represented as i32 in prost
        if rust_field.is_enum {
            trace.checkpoint_data(
                "enum_type_detected_not_optional",
                &[
                    ("type_name", &rust_field.type_name),
                    ("is_enum", &rust_field.is_enum.to_string()),
                ]
            );
            return false; // Enums are required (i32) by default
        }

        // Transparent fields that wrap primitives typically map to
        // required proto fields (u64, not Option<u64>)
        if rust_field.has_transparent {
            trace.checkpoint_data(
                "transparent_field_not_optional",
                &[
                    ("type_name", &rust_field.type_name),
                    ("has_transparent", &rust_field.has_transparent.to_string()),
                ]
            );
            return false; // Transparent fields are typically required
        }

        // Pattern: Custom type in Rust that's not wrapped in Option<>
        // but appears in proto as optional (common pattern)
        let is_unwrapped_custom = !rust_field.is_option && rust_field.is_custom;
        if is_unwrapped_custom {
            trace.checkpoint_data(
                "custom_type_pattern_detected",
                &[
                    ("type_name", &rust_field.type_name),
                    ("is_option", &rust_field.is_option.to_string()),
                    ("is_custom", &rust_field.is_custom.to_string()),
                ]
            );
        }

        is_unwrapped_custom
    }

    /// Check for strong indicators that proto field should be optional
    fn has_strong_optional_indicators(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> bool {
        // Vec fields should NEVER have strong optional indicators
        // regardless of default attributes, since they map to protobuf repeated fields
        if rust_field.is_vec {
            trace.decision("is_vec", "Vec<T> -> repeated proto field, never optional");
            return false;
        }

        let has_expect = !matches!(ctx.expect_mode, ExpectMode::None);
        let has_default = ctx.has_default;
        let has_transparent_with_expect = rust_field.has_transparent && has_expect;

        let result = has_expect || has_default || has_transparent_with_expect;
        if result {
            trace.checkpoint_data(
                "strong_optional_indicators_found",
                &[
                    ("has_expect", &has_expect.to_string()),
                    ("has_default", &has_default.to_string()),
                    ("has_transparent_with_expect", &has_transparent_with_expect.to_string()),
                ]
            );
        }

        result
    }

    /// Check for any indicators of optional usage
    fn has_any_optional_indicators(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
    ) -> bool {
        !matches!(ctx.expect_mode, ExpectMode::None) || ctx.has_default
    }


    fn infer_proto_type_name(ctx: &FieldProcessingContext, rust_field: &RustFieldInfo) -> String {
        if rust_field.has_transparent {
            // transparent fields use inner type
            "inner_type".to_string()
        } else if rust_field.is_custom && !Self::is_likely_proto_type(ctx, rust_field) {
            // custom type may map to proto message types
            if rust_field.type_name.contains("::") {
                rust_field.type_name.clone()
            } else {
                format!("{}::{}", ctx.proto_module, rust_field.type_name)
            }
        } else {
            // primitives map directly
            rust_field.type_name.clone()
        }
    }

    // further proto type detection to avoid double-prefixing and improve resilience
    fn is_likely_proto_type(ctx: &FieldProcessingContext, rust_field: &RustFieldInfo) -> bool {
        // Check for explicit proto module prefixes
        if rust_field.type_name.starts_with(&format!("{}::", ctx.proto_module)) ||
            rust_field.type_name.starts_with("proto::") {
            return true;
        }

        // More resilient detection - parse as syn::Type and check path segments
        if let Ok(parsed_type) = syn::parse_str::<syn::Type>(&rust_field.type_name) {
            if let syn::Type::Path(type_path) = parsed_type {
                // Check if any segment matches the proto module
                return type_path.path.segments.iter().any(|segment| {
                    segment.ident == ctx.proto_module || segment.ident == "proto"
                });
            }
        }

        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConversionStrategy {
    // Ignore fields
    ProtoIgnore,

    // Custom derive functions
    DeriveFromWith(String),
    DeriveIntoWith(String),
    DeriveBidirectional(String, String),    // (from_with_path, into_with_path)

    // Transparent field handling
    TransparentRequired,                    // proto field is required
    TransparentOptionalWithExpect,          // proto optional -> rust required (expect)
    TransparentOptionalWithError,           // proto optional -> rust required (error)
    TransparentOptionalWithDefault,         // proto optional -> rust required (default)
    TransparentToOptional,                  // rust -> proto optional (rust_to_proto)
    TransparentToRequired,                  // rust -> proto required (rust_to_proto)

    // Direct conversions (no wrapping/unwrapping)
    DirectAssignment,                       // T -> T (primitives, proto types)
    DirectWithInto,                         // CustomType -> ProtoType

    // Option handling
    WrapInSome,                            // T -> Some(T) (rust required -> proto optional)
    UnwrapOptional,                        // Option<T> -> T (for rust_to_proto)
    UnwrapOptionalWithExpect,              // Option<T> -> T (with expect)
    UnwrapOptionalWithError,               // Option<T> -> T (with error handling)
    UnwrapOptionalWithDefault,             // Option<T> -> T (with default fallback)
    MapOption,                             // Option<T> -> Option<U>
    MapOptionWithDefault,                  // Option<T> -> Some(t)

    // Collection handling
    CollectVec,                            // Vec<T> -> Vec<U>
    CollectVecWithDefault,                 // Vec<T> -> Vec<U> (with default when empty)
    CollectVecWithError,                   // Vec<T> -> Vec<U> (with error handling)
    MapVecInOption,                        // Option<Vec<T>> -> Option<Vec<U>>
    VecDirectAssignment,                   // Vec<ProtoType> -> Vec<ProtoType> (no conversion needed)

    // Error cases
    RequiresCustomLogic,                    // Complex cases needing manual handling
}

impl ConversionStrategy {
    /// Debug enhancement for better tracing
    pub fn debug_info(&self) -> &'static str {
        match self {
            Self::ProtoIgnore => "field ignored - not in proto",
            Self::DeriveFromWith(_) => "custom from function",
            Self::DeriveIntoWith(_) => "custom into function",
            Self::DeriveBidirectional(_, _) => "custom from and into functions",
            Self::TransparentRequired => "transparent field, proto required",
            Self::TransparentOptionalWithExpect => "transparent field, proto optional -> expect",
            Self::TransparentOptionalWithError => "transparent field, proto optional -> error",
            Self::TransparentOptionalWithDefault => "transparent field, proto optional -> default",
            Self::TransparentToOptional => "transparent field -> proto optional",
            Self::TransparentToRequired => "transparent field -> proto required",
            Self::DirectAssignment => "direct assignment (no conversion)",
            Self::DirectWithInto => "direct conversion with Into",
            Self::WrapInSome => "wrap value in Some()",
            Self::UnwrapOptional => "unwrap option (rust_to_proto)",
            Self::UnwrapOptionalWithExpect => "proto optional -> rust required (expect)",
            Self::UnwrapOptionalWithError => "proto optional -> rust required (error)",
            Self::UnwrapOptionalWithDefault => "proto optional -> rust required (default)",
            Self::MapOptionWithDefault => "proto optional -> rust optional with default",
            Self::MapOption => "map option through conversion",
            Self::CollectVec => "collect vector with conversion",
            Self::CollectVecWithDefault => "collect vector with default fallback",
            Self::CollectVecWithError => "collect vector with error handling",
            Self::MapVecInOption => "map optional vector with conversion",
            Self::VecDirectAssignment => "direct vector assignment (proto types)",
            Self::RequiresCustomLogic => "complex case requiring manual handling",
        }
    }

    /// Strategy category for grouping related strategies
    pub fn category(&self) -> &'static str {
        match self {
            Self::ProtoIgnore => "ignore",

            Self::DeriveFromWith(_) | Self::DeriveIntoWith(_) | Self::DeriveBidirectional(_, _) => "custom_derive",

            Self::TransparentRequired | Self::TransparentOptionalWithExpect |
            Self::TransparentOptionalWithError | Self::TransparentOptionalWithDefault |
            Self::TransparentToOptional | Self::TransparentToRequired => "transparent",

            Self::DirectAssignment | Self::DirectWithInto => "direct",

            Self::WrapInSome | Self::UnwrapOptional | Self::UnwrapOptionalWithExpect |
            Self::UnwrapOptionalWithError | Self::UnwrapOptionalWithDefault |
            Self::MapOption | Self::MapOptionWithDefault => "option",

            Self::CollectVec | Self::CollectVecWithDefault | Self::CollectVecWithError |
            Self::MapVecInOption | Self::VecDirectAssignment => "collection",

            Self::RequiresCustomLogic => "error",
        }
    }

    // DMR: Validation to catch impossible strategy combinations early
    pub fn validate_for_context(
        &self,
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
    ) -> Result<(), String> {
        match self {
            // -- Validate unwrap strategies require proto optional --
            Self::UnwrapOptionalWithExpect | Self::UnwrapOptionalWithError | Self::UnwrapOptionalWithDefault => {
                if !proto.is_optional() {
                    return Err(format!(
                        "UnwrapOptional strategy '{}' requires proto field to be optional, but detected as required. \
                         Add #[proto(proto_optional)] if proto field is actually optional.",
                        self.debug_info()
                    ));
                }
            },

            // -- Validate wrap strategies require rust non-optional to proto optional --
            Self::WrapInSome => {
                if rust.is_option {
                    return Err(format!(
                        "WrapInSome strategy incompatible with rust Option type. Use MapOption instead."
                    ));
                }
                if !proto.is_optional() {
                    return Err(format!(
                        "WrapInSome strategy requires proto field to be optional, but detected as required."
                    ));
                }
            },

            // -- Validate map option requires at least one side to be optional --
            Self::MapOption => {
                if !rust.is_option && !proto.is_optional() {
                    return Err(format!(
                        "MapOption strategy requires at least one side to be Option type. \
                         Rust: {}, Proto: {} - use DirectWithInto instead.",
                        if rust.is_option { "Option" } else { "Required" },
                        if proto.is_optional() { "Option" } else { "Required" }
                    ));
                }
            },

            // -- Validate transparent strategies require transparent attribute --
            Self::TransparentRequired | Self::TransparentOptionalWithExpect |
            Self::TransparentOptionalWithError | Self::TransparentOptionalWithDefault |
            Self::TransparentToOptional | Self::TransparentToRequired => {
                if !rust.has_transparent {
                    return Err(format!(
                        "Transparent strategy '{}' requires #[proto(transparent)] attribute on field.",
                        self.debug_info()
                    ));
                }
            },

            // -- Validate vector strategies require vector types --
            Self::CollectVec | Self::CollectVecWithDefault | Self::CollectVecWithError | Self::VecDirectAssignment => {
                if !rust.is_vec {
                    return Err(format!(
                        "Vector strategy '{}' requires rust field to be Vec<T> type.",
                        self.debug_info()
                    ));
                }
            },

            // -- Validate option vec strategy --
            Self::MapVecInOption => {
                if !Self::is_option_vec_type(ctx.field_type) {
                    return Err(format!(
                        "MapVecInOption strategy requires Option<Vec<T>> type, found: {}",
                        quote!(ctx.field_type).to_string()
                    ));
                }
            },

            // -- Validate derive strategies have paths --
            Self::DeriveFromWith(path) | Self::DeriveIntoWith(path) => {
                if path.is_empty() {
                    return Err(format!("Derive strategy requires non-empty function path"));
                }
                //todo: Could add path validation here
            },

            // -- Validate ignore strategy --
            Self::ProtoIgnore => {
                if !rust.has_proto_ignore {
                    return Err(format!(
                        "ProtoIgnore strategy requires #[proto(ignore)] attribute on field."
                    ));
                }
            },

            // -- Other strategies are always valid --
            _ => {}
        }

        Ok(())
    }

    pub fn from_field_info(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo
    ) -> Self {
        let _trace = CallStackDebug::with_context(
            "ConversionStrategy::from_field_info",
            ctx.struct_name,
            ctx.field_name,
            &[
                ("rust_is_option", &rust.is_option.to_string()),
                ("rust_is_vec", &rust.is_vec.to_string()),
                ("proto_is_option", &proto.is_optional().to_string()),
                ("proto_is_vec", &proto.is_vec.to_string()),
                ("proto_optionality", &format!("{:?}", proto.optionality)),
            ],
        );

        // -- Handle special cases first --
        let result = if rust.has_proto_ignore {
            _trace.decision("has_proto_ignore", "ProtoIgnore");
            Self::ProtoIgnore
        } else if let (Some(from_with_path), Some(into_with_path)) = (&rust.derive_from_with, &rust.derive_into_with) {
            _trace.decision("has_both_derive_functions", "DeriveBidirectional");
            Self::DeriveBidirectional(from_with_path.clone(), into_with_path.clone())
        } else if let Some(from_with_path) = &rust.derive_from_with {
            _trace.decision("has_derive_from_with", "DeriveFromWith");
            Self::DeriveFromWith(from_with_path.clone())
        } else if let Some(into_with_path) = &rust.derive_into_with {
            _trace.decision("has_derive_into_with", "DeriveIntoWith");
            Self::DeriveIntoWith(into_with_path.clone())
        } else if rust.has_transparent {
            // -- Handle transparent fields --
            Self::determine_transparent_strategy(ctx, rust, proto, &_trace)
        } else if rust.is_vec || Self::is_option_vec_type(ctx.field_type) {
            // -- Handle collections (including Option<Vec<T>>) --
            Self::determine_vec_strategy(ctx, rust, proto, &_trace)
        } else if rust.is_option || proto.is_optional() {
            // -- Handle simple options (after vec check since Option<Vec<T>> is handled above) --
            Self::determine_option_strategy(ctx, rust, proto, &_trace)
        } else {
            // -- Handle direct conversions --
            Self::determine_direct_strategy(ctx, rust, proto, &_trace)
        };

        // _trace.checkpoint_data(
        //     "conversion_strategy",
        //     &[
        //         ("category", result.category()),
        //         ("info", result.debug_info()),
        //     ]
        // );

        result
    }

    // Helper to detect Option<Vec<T>> pattern
    fn is_option_vec_type(field_type: &syn::Type) -> bool {
        type_analysis::get_inner_type_from_option(field_type)
            .map(|inner_type| type_analysis::is_vec_type(&inner_type))
            .unwrap_or(false)
    }

    fn determine_transparent_strategy(
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
        trace: &CallStackDebug,
    ) -> Self {
        match proto.optionality {
            FieldOptionality::Required => {
                trace.decision("transparent + proto_required", "TransparentRequired");
                Self::TransparentRequired
            },
            FieldOptionality::Optional => {
                match rust.expect_mode {
                    ExpectMode::Panic => {
                        trace.decision("transparent + proto_optional + expect_panic", "TransparentOptionalWithExpect");
                        Self::TransparentOptionalWithExpect
                    },
                    ExpectMode::Error => {
                        trace.decision("transparent + proto_optional + expect_error", "TransparentOptionalWithError");
                        Self::TransparentOptionalWithError
                    },
                    ExpectMode::None => {
                        if rust.has_default {
                            trace.decision("transparent + proto_optional + has_default", "TransparentOptionalWithDefault");
                            Self::TransparentOptionalWithDefault
                        } else {
                            trace.decision("transparent + proto_optional + no_expect + no_default", "TransparentRequired");
                            Self::TransparentRequired
                        }
                    }
                }
            }
        }
    }

    fn determine_vec_strategy(
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
        trace: &CallStackDebug,
    ) -> Self {
        // -- Handle Option<Vec<T>> case first --
        if Self::is_option_vec_type(ctx.field_type) {
            trace.decision("option_vec_type", "MapVecInOption");
            return Self::MapVecInOption;
        }

        // -- Enhanced vector strategy determination with better proto type detection --
        if let Some(inner_type) = type_analysis::get_inner_type_from_vec(ctx.field_type) {
            let is_proto_inner = type_analysis::is_proto_type(&inner_type, ctx.proto_module);

            if is_proto_inner {
                trace.decision("vec + proto_inner_type", "VecDirectAssignment");
                return Self::VecDirectAssignment;
            }
        }

        if rust.has_default {
            match rust.expect_mode {
                ExpectMode::Error => {
                    trace.decision("vec + has_default + expect_error", "CollectVecWithError");
                    Self::CollectVecWithError
                },
                _ => {
                    trace.decision("vec + has_default", "CollectVecWithDefault");
                    Self::CollectVecWithDefault
                }
            }
        } else {
            trace.decision("vec + no_default", "CollectVec");
            Self::CollectVec
        }
    }

    fn determine_option_strategy(
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
        trace: &CallStackDebug,
    ) -> Self {
        match (rust.is_option, proto.is_optional(), rust.expect_mode) {
            (false, true, ExpectMode::Panic) => {
                trace.decision("rust_required + proto_optional + expect_panic", "UnwrapOptionalWithExpect");
                Self::UnwrapOptionalWithExpect
            },
            (false, true, ExpectMode::Error) => {
                trace.decision("rust_required + proto_optional + expect_error", "UnwrapOptionalWithError");
                Self::UnwrapOptionalWithError
            },
            (false, true, ExpectMode::None) if rust.has_default => {
                trace.decision("rust_required + proto_optional + has_default", "UnwrapOptionalWithDefault");
                Self::UnwrapOptionalWithDefault
            },
            (false, true, ExpectMode::None) => {
                trace.decision("rust_required + proto_optional + no_expect + no_default", "UnwrapOptionalWithExpect");
                Self::UnwrapOptionalWithExpect
            },
            (true, false, _) => {
                // -- Rust optional, Proto required - need to wrap or unwrap depending on direction --
                trace.decision("rust_optional + proto_required", "WrapInSome");
                Self::WrapInSome
            },
            (true, true, ExpectMode::Panic) => {
                trace.decision("both_optional + expect_panic", "UnwrapOptionalWithExpect");
                Self::UnwrapOptionalWithExpect
            },
            (true, true, ExpectMode::Error) => {
                trace.decision("both_optional + expect_error", "UnwrapOptionalWithError");
                Self::UnwrapOptionalWithError
            },

            (true, true, ExpectMode::None) if rust.has_default => {
                trace.decision("both_optional + no_expect + has_default", "MapOptionWithDefault");
                Self::MapOptionWithDefault
            },
            (true, true, ExpectMode::None) => {
                // -- Both optional with no expect - map through --
                trace.decision("both_optional + no_expect", "MapOption");
                Self::MapOption
            },

            (true, true, ExpectMode::None) => {
                // -- Both optional with no expect - map through --
                trace.decision("both_optional + no_expect", "MapOption");
                Self::MapOption
            },

            (false, false, _) => {
                // -- Both required - direct conversion --
                trace.decision("both_required", "DirectWithInto");
                Self::DirectWithInto
            },
        }
    }

    fn determine_direct_strategy(
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
        trace: &CallStackDebug,
    ) -> Self {
        // -- direct conversion strategy determination --
        if rust.is_primitive && proto.is_primitive {
            trace.decision("both_primitive", "DirectAssignment");
            Self::DirectAssignment
        } else if Self::is_proto_type_conversion(ctx, rust) {
            // -- Proto type to proto type conversions --
            trace.decision("proto_type_conversion", "DirectAssignment");
            Self::DirectAssignment
        } else {
            trace.decision("requires_conversion", "DirectWithInto");
            Self::DirectWithInto
        }
    }

    // proto type detection - more resilient than just checking first segment
    fn is_proto_type_conversion(ctx: &FieldProcessingContext, rust: &RustFieldInfo) -> bool {
        // Check if this is a conversion between proto module types
        syn::parse_str::<syn::Type>(&rust.type_name)
            .map(|field_type_rep| type_analysis::is_proto_type(&field_type_rep, ctx.proto_module))
            .unwrap_or(false)
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

        let proto_field_ident = attribute_parser::get_proto_rename(field)
            .map(|rename| syn::Ident::new(&rename, proc_macro2::Span::call_site()))
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
