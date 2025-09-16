use crate::analysis::field_analysis::{self, FieldProcessingContext};
use crate::conversion::ConversionStrategy as OldConversionStrategy;
use crate::field::{
    conversion_strategy::{FieldConversionStrategy, },
    info::{ProtoFieldInfo, RustFieldInfo},
};

/// Results from running both old and new strategy selection systems
#[derive(Debug, Clone)]
pub struct StrategyComparisonResult {
    pub old_strategy: OldConversionStrategy,
    pub new_strategy: FieldConversionStrategy,
    pub strategies_match: bool,
    pub old_from_proto: proc_macro2::TokenStream,
    pub old_to_proto: proc_macro2::TokenStream,
    pub new_from_proto: proc_macro2::TokenStream,
    pub new_to_proto: proc_macro2::TokenStream,
    pub from_proto_generation_matches: bool,
    pub to_proto_generation_matches: bool,
    pub field_name: String,
    pub struct_name: String,
}

/// Test harness for comparing old and new strategy systems
pub struct StrategyCompatibilityTester;

impl StrategyCompatibilityTester {
    /// Compare strategy selection for a single field using both systems
    pub fn compare_field_strategies(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
    ) -> Result<StrategyComparisonResult, String> {
        // Run old system
        let rust_field = RustFieldInfo::analyze(ctx, field);
        let proto_field = ProtoFieldInfo::infer_from(ctx, field, &rust_field);
        let old_strategy =
            OldConversionStrategy::from_field_info(ctx, field, &rust_field, &proto_field);

        // Generate old code
        let (old_from_proto, old_to_proto) =
            field_analysis::generate_field_conversions(field, ctx)
                .map_err(|e| format!("Old system code generation failed: {:?}", e))?;

        // Run new system
        let new_strategy =
            FieldConversionStrategy::from_field_info(ctx, field, &rust_field, &proto_field);

        // Generate new code (placeholder - we'll implement this in later steps)
        let (new_from_proto, new_to_proto) =
            Self::generate_new_field_conversions(&new_strategy, ctx, field);

        // Compare strategies
        let strategies_match = Self::strategies_are_equivalent(&old_strategy, &new_strategy);

        // Compare generated code
        let from_proto_generation_matches = Self::compare_generated_code(
            "from_proto",
            &old_from_proto,
            &new_from_proto
        );
        let to_proto_generation_matches = Self::compare_generated_code(
            "to_proto",
            &old_to_proto,
            &new_to_proto
        );

        Ok(StrategyComparisonResult {
            old_strategy,
            new_strategy,
            strategies_match,
            old_from_proto,
            old_to_proto,
            new_from_proto,
            new_to_proto,
            from_proto_generation_matches,
            to_proto_generation_matches,
            field_name: ctx.field_name.to_string(),
            struct_name: ctx.struct_name.to_string(),
        })
    }

    /// Check if old and new strategies are equivalent
    fn strategies_are_equivalent(
        old: &OldConversionStrategy,
        new: &FieldConversionStrategy,
    ) -> bool {
        // Try to map old strategy to new and see if it matches
        if let Some(mapped_new) = FieldConversionStrategy::from_old_strategy(old)
        && &mapped_new == new {
            true
        } else if let Some(mapped_old) = new.to_old_strategy() && &mapped_old == old {
            true
        } else {
            // If we can't map the old strategy, they don't match
            false
        }
    }

    /// Compare generated code (simplified comparison for now)
    fn compare_generated_code(
        label: &str,
        old: &proc_macro2::TokenStream,
        new: &proc_macro2::TokenStream
    ) -> bool {
        // For now, just compare the string representations
        // In a real implementation, you might want more sophisticated comparison
        let old_str = Self::normalize_code(&old.to_string());
        let new_str = Self::normalize_code(&new.to_string());

        eprintln!("{label}\nold_str: {old_str}\nnew_str: {new_str}\n");
        old_str == new_str
    }

    /// Normalize code for comparison by removing extra whitespace
    fn normalize_code(code: &str) -> String {
        code.split_whitespace().collect::<Vec<_>>().join("")
    }

    /// Generate field conversions using new strategy (updated implementation)
    pub fn generate_new_field_conversions(
        strategy: &FieldConversionStrategy,
        ctx: &FieldProcessingContext,
        field: &syn::Field,
    ) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
        let proto_to_rust = strategy.generate_proto_to_rust_conversion(ctx, field);
        let rust_to_proto = strategy.generate_rust_to_proto_conversion(ctx, field);
        (proto_to_rust, rust_to_proto)
    }

    /// Run compatibility tests on multiple fields and generate a report
    pub fn run_compatibility_report(
        fields_and_contexts: Vec<(&syn::Field, &FieldProcessingContext)>,
    ) -> CompatibilityReport {
        let mut results = Vec::new();
        let mut total_fields = 0;
        let mut strategy_matches = 0;
        let mut code_matches = 0;
        let mut failures = Vec::new();

        for (field, ctx) in fields_and_contexts {
            total_fields += 1;

            match Self::compare_field_strategies(ctx, field) {
                Ok(result) => {
                    if result.strategies_match {
                        strategy_matches += 1;
                    }
                    if result.from_proto_generation_matches && result.to_proto_generation_matches {
                        code_matches += 1;
                    }
                    results.push(result);
                }
                Err(error) => {
                    failures.push(FieldComparisonFailure {
                        field_name: ctx.field_name.to_string(),
                        struct_name: ctx.struct_name.to_string(),
                        error,
                    });
                }
            }
        }

        CompatibilityReport {
            total_fields,
            strategy_matches,
            code_matches,
            results,
            failures,
        }
    }
}

/// Report from running compatibility tests across multiple fields
#[derive(Debug)]
pub struct CompatibilityReport {
    pub total_fields: usize,
    pub strategy_matches: usize,
    pub code_matches: usize,
    pub results: Vec<StrategyComparisonResult>,
    pub failures: Vec<FieldComparisonFailure>,
}

#[derive(Debug)]
pub struct FieldComparisonFailure {
    pub field_name: String,
    pub struct_name: String,
    pub error: String,
}

impl CompatibilityReport {
    /// Print a summary of the compatibility test results
    pub fn print_summary(&self) {
        println!("=== Strategy Compatibility Report ===");
        println!("Total fields tested: {}", self.total_fields);
        println!(
            "Strategy matches: {}/{} ({:.1}%)",
            self.strategy_matches,
            self.total_fields,
            (self.strategy_matches as f64 / self.total_fields as f64) * 100.0
        );
        println!(
            "Code generation matches: {}/{} ({:.1}%)",
            self.code_matches,
            self.total_fields,
            (self.code_matches as f64 / self.total_fields as f64) * 100.0
        );

        if !self.failures.is_empty() {
            println!("\nFailures:");
            for failure in &self.failures {
                println!(
                    "  {}.{}: {}",
                    failure.struct_name, failure.field_name, failure.error
                );
            }
        }

        if self.strategy_matches < self.total_fields {
            println!("\nStrategy mismatches:");
            for result in &self.results {
                if !result.strategies_match {
                    println!(
                        "  {}.{}: {:?} -> {:?}",
                        result.struct_name,
                        result.field_name,
                        result.old_strategy,
                        result.new_strategy
                    );
                }
            }
        }
    }

    /// Get fields where strategies don't match
    pub fn get_strategy_mismatches(&self) -> Vec<&StrategyComparisonResult> {
        self.results
            .iter()
            .filter(|r| !r.strategies_match)
            .collect()
    }

    /// Get fields where code generation doesn't match
    pub fn get_code_mismatches(&self) -> Vec<&StrategyComparisonResult> {
        self.results
            .iter()
            .filter(|r| !r.from_proto_generation_matches || !r.to_proto_generation_matches)
            .collect()
    }

    /// Check if all tests passed
    pub fn all_tests_passed(&self) -> bool {
        self.failures.is_empty()
            && self.strategy_matches == self.total_fields
            && self.code_matches == self.total_fields
    }
}

#[cfg(test)]
mod tests {
    use crate::field::conversion_strategy;
    use super::*;

    #[test]
    fn test_strategy_equivalence_detection() {
        let old_direct = OldConversionStrategy::DirectAssignment;
        let new_direct =
            FieldConversionStrategy::Direct(conversion_strategy::DirectStrategy::Assignment);

        assert!(StrategyCompatibilityTester::strategies_are_equivalent(
            &old_direct,
            &new_direct
        ));
    }

    #[test]
    fn test_code_normalization() {
        let code1 = "field_name : proto_struct . field_name . into ( )";
        let code2 = "field_name: proto_struct.field_name.into()";

        assert_eq!(
            StrategyCompatibilityTester::normalize_code(code1),
            StrategyCompatibilityTester::normalize_code(code2)
        );
    }

    // Add more tests as you develop the system
    #[test]
    fn test_compatibility_report_creation() {
        let report = CompatibilityReport {
            total_fields: 10,
            strategy_matches: 8,
            code_matches: 7,
            results: vec![],
            failures: vec![],
        };

        assert!(!report.all_tests_passed()); // Not all code matches
        assert_eq!(report.get_strategy_mismatches().len(), 0); // No results to check
    }

    #[test]
    fn test_comprehensive_compatibility() {
        let test_suite = test_helpers::create_comprehensive_test_suite();
        let report = test_suite.run_all_tests();

        report.print_summary();

        if !report.all_tests_passed() {
            println!("Issues found:");
            for mismatch in report.get_strategy_mismatches() {
                println!(
                    "Strategy mismatch: {}.{}",
                    mismatch.struct_name, mismatch.field_name
                );
            }
            for mismatch in report.get_code_mismatches() {
                println!(
                    "Code mismatch: {}.{}",
                    mismatch.struct_name, mismatch.field_name
                );
            }
        }
    }
}

// Helper functions for creating test contexts and mock data
#[cfg(test)]
pub mod test_helpers {
    use super::*;
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
            &None, // struct_level_error_type
            &None, // struct_level_error_fn
            proto_module_static,
            proto_name_static,
        );

        (field, context)
    }

    /// Create test cases based on patterns observed in your test files
    /// This extracts common field patterns from your actual tests
    pub fn create_common_test_cases() -> Vec<(syn::Field, FieldProcessingContext<'static>)> {
        let mut test_cases = Vec::new();

        // Basic primitive fields (from basic_tests.rs)
        test_cases.push(create_mock_context(
            "Track",
            "track_id",
            "u64",
            "proto",
            &[],
        ));
        test_cases.push(create_mock_context(
            "StatusResponse",
            "message",
            "String",
            "proto",
            &[],
        ));

        // Optional fields (from basic_tests.rs - HasOptional)
        test_cases.push(create_mock_context(
            "HasOptional",
            "track",
            "Option<Track>",
            "proto",
            &[],
        ));

        // Fields with default (from default_tests.rs - TrackWithDefault)
        test_cases.push(create_mock_context(
            "TrackWithDefault",
            "name",
            "String",
            "proto",
            &["default"],
        ));
        test_cases.push(create_mock_context(
            "TrackWithDefault",
            "duration",
            "u32",
            "proto",
            &["default"],
        ));

        // Custom default functions (from default_tests.rs - TrackWithCustomDefault)
        test_cases.push(create_mock_context(
            "TrackWithCustomDefault",
            "name",
            "String",
            "proto",
            &["default = \"default_track_name\""],
        ));
        test_cases.push(create_mock_context(
            "TrackWithCustomDefault",
            "duration",
            "u32",
            "proto",
            &["default = \"default_duration\""],
        ));

        // Collection fields (from basic_tests.rs - State)
        test_cases.push(create_mock_context(
            "State",
            "tracks",
            "Vec<Track>",
            "proto",
            &[],
        ));

        // Transparent wrappers (from basic_types - Track.id field)
        test_cases.push(create_mock_context(
            "Track",
            "id",
            "TrackId",
            "proto",
            &["transparent"],
        ));

        // Error handling fields (from error_tests.rs)
        test_cases.push(create_mock_context(
            "HasOptionalWithError",
            "track",
            "Option<Track>",
            "proto",
            &["expect"],
        ));
        test_cases.push(create_mock_context(
            "ComplexExpectStruct",
            "field_with_panic",
            "String",
            "proto",
            &["expect(panic)"],
        ));
        test_cases.push(create_mock_context(
            "ComplexExpectStruct",
            "field_with_error",
            "String",
            "proto",
            &["expect", "error_fn = \"ValidationError::missing_field\""],
        ));

        // Custom conversion functions (from advanced_tests.rs - BidirectionalConversionStruct)
        test_cases.push(create_mock_context(
            "BidirectionalConversionStruct",
            "custom_field",
            "CustomComplexType",
            "proto",
            &[
                "proto_to_rust_fn = \"custom_from_conversion\"",
                "rust_to_proto_fn = \"custom_into_conversion\"",
            ],
        ));

        // Renamed fields (from integration_tests.rs - CombinationStruct)
        test_cases.push(create_mock_context(
            "CombinationStruct",
            "renamed_field_with_default",
            "String",
            "proto",
            &[
                "proto_name = \"rename_with_default\"",
                "default = \"renamed_default\"",
            ],
        ));

        // Enum fields (from basic_tests.rs - StatusResponse.status)
        test_cases.push(create_mock_context(
            "StatusResponse",
            "status",
            "Status",
            "proto",
            &[],
        ));
        test_cases.push(create_mock_context(
            "ComprehensiveEnumStruct",
            "enum_expect_panic",
            "Status",
            "proto",
            &["expect(panic)"],
        ));
        test_cases.push(create_mock_context(
            "ComprehensiveEnumStruct",
            "enum_with_default",
            "Status",
            "proto",
            &["default = \"default_status\""],
        ));

        // Ignored fields (from advanced_tests.rs - ComplexState)
        test_cases.push(create_mock_context(
            "ComplexState",
            "launches",
            "HashMap<String, String>",
            "proto",
            &["ignore"],
        ));
        test_cases.push(create_mock_context(
            "ComplexState",
            "counter",
            "AtomicU64",
            "proto",
            &["ignore"],
        ));

        // Optional collections (from advanced_tests.rs - VecOptionStruct)
        test_cases.push(create_mock_context(
            "VecOptionStruct",
            "optional_tracks",
            "Option<Vec<Track>>",
            "proto",
            &[],
        ));

        // Transparent with expect (from integration_tests.rs - CombinationStruct)
        test_cases.push(create_mock_context(
            "CombinationStruct",
            "transparent_field_with_expect",
            "TransparentWrapper",
            "proto",
            &["transparent", "expect"],
        ));

        test_cases
    }

    /// Create specific test case for a known pattern from your tests
    pub fn create_test_case_for_pattern(
        pattern_name: &str,
    ) -> Option<(syn::Field, FieldProcessingContext<'static>)> {
        match pattern_name {
            "basic_primitive" => Some(create_mock_context(
                "Track",
                "track_id",
                "u64",
                "proto",
                &[],
            )),
            "optional_with_expect" => Some(create_mock_context(
                "HasOptionalWithError",
                "track",
                "Option<Track>",
                "proto",
                &["expect"],
            )),
            "transparent_wrapper" => Some(create_mock_context(
                "Track",
                "id",
                "TrackId",
                "proto",
                &["transparent"],
            )),
            "vec_collection" => Some(create_mock_context(
                "State",
                "tracks",
                "Vec<Track>",
                "proto",
                &[],
            )),
            "custom_bidirectional" => Some(create_mock_context(
                "BidirectionalConversionStruct",
                "custom_field",
                "CustomComplexType",
                "proto",
                &[
                    "proto_to_rust_fn = \"custom_from_conversion\"",
                    "rust_to_proto_fn = \"custom_into_conversion\"",
                ],
            )),
            "ignored_field" => Some(create_mock_context(
                "ComplexState",
                "launches",
                "HashMap<String, String>",
                "proto",
                &["ignore"],
            )),
            "expect_panic" => Some(create_mock_context(
                "ComplexExpectStruct",
                "field_with_panic",
                "String",
                "proto",
                &["expect(panic)"],
            )),
            "expect_error" => Some(create_mock_context(
                "ComplexExpectStruct",
                "field_with_error",
                "String",
                "proto",
                &["expect", "error_fn = \"ValidationError::missing_field\""],
            )),
            "expect_with_custom_error" => Some(create_mock_context(
                "ComplexExpectStruct",
                "field_with_custom_error",
                "String",
                "proto",
                &["expect", "error_fn = \"ValidationError::invalid_value\""],
            )),
            "expect_with_default" => Some(create_mock_context(
                "ComplexExpectStruct",
                "number_with_default",
                "u64",
                "proto",
                &["default = \"default_number\""],
            )),
            "enum_comprehensive" => Some(create_mock_context(
                "ComprehensiveEnumStruct",
                "enum_with_default",
                "Status",
                "proto",
                &["default = \"default_status\""],
            )),
            "transparent_optional_strategies" => Some(create_mock_context(
                "TransparentOptionalStruct",
                "panic_wrapper",
                "TransparentWrapper",
                "proto",
                &["transparent", "expect(panic)"],
            )),
            "map_option" => Some(create_mock_context(
                "MapOptionStruct",
                "optional_string",
                "Option<String>",
                "proto",
                &[],
            )),
            "vec_with_error" => Some(create_mock_context(
                "VecWithErrorStruct",
                "tracks_with_error",
                "Vec<Track>",
                "proto",
                &["expect", "error_fn = \"default_track_vec\""],
            )),
            "bidirectional_conversion" => Some(create_mock_context(
                "BidirectionalConversionStruct",
                "custom_field",
                "CustomComplexType",
                "proto",
                &[
                    "proto_to_rust_fn = \"custom_from_conversion\"",
                    "rust_to_proto_fn = \"custom_into_conversion\"",
                ],
            )),
            _ => None,
        }
    }

    /// Run compatibility test on a specific pattern
    pub fn test_pattern_compatibility(
        pattern_name: &str,
    ) -> Result<StrategyComparisonResult, String> {
        let (field, context) = create_test_case_for_pattern(pattern_name)
            .ok_or_else(|| format!("Unknown pattern: {}", pattern_name))?;

        StrategyCompatibilityTester::compare_field_strategies(&context, &field)
    }

    /// Create a comprehensive test suite based on your test files
    pub fn create_comprehensive_test_suite() -> CompatibilityTestSuite {
        CompatibilityTestSuite {
            basic_patterns: vec![
                "basic_primitive",
                "optional_with_expect",
                "transparent_wrapper",
                "vec_collection",
                "custom_bidirectional",
                "ignored_field",
            ],
            error_patterns: vec![
                "expect_panic",
                "expect_error",
                "expect_with_custom_error",
                "expect_with_default",
            ],
            complex_patterns: vec![
                "bidirectional_conversion",
                "transparent_optional_strategies",
                "map_option",
                "vec_with_error",
                "enum_comprehensive",
            ],
        }
    }

    /// Test suite structure for organizing compatibility tests
    pub struct CompatibilityTestSuite {
        pub basic_patterns: Vec<&'static str>,
        pub error_patterns: Vec<&'static str>,
        pub complex_patterns: Vec<&'static str>,
    }

    impl CompatibilityTestSuite {
        /// Run all tests in the suite
        pub fn run_all_tests(&self) -> CompatibilityReport {
            let mut all_results = Vec::new();
            let mut failures = Vec::new();

            for &pattern in &self.basic_patterns {
                match test_helpers::test_pattern_compatibility(pattern) {
                    Ok(result) => all_results.push(result),
                    Err(error) => failures.push(FieldComparisonFailure {
                        field_name: pattern.to_string(),
                        struct_name: "TestStruct".to_string(),
                        error,
                    }),
                }
            }

            for &pattern in &self.error_patterns {
                match test_helpers::test_pattern_compatibility(pattern) {
                    Ok(result) => all_results.push(result),
                    Err(error) => failures.push(FieldComparisonFailure {
                        field_name: pattern.to_string(),
                        struct_name: "ErrorTestStruct".to_string(),
                        error,
                    }),
                }
            }

            for &pattern in &self.complex_patterns {
                match test_helpers::test_pattern_compatibility(pattern) {
                    Ok(result) => all_results.push(result),
                    Err(error) => failures.push(FieldComparisonFailure {
                        field_name: pattern.to_string(),
                        struct_name: "ComplexTestStruct".to_string(),
                        error,
                    }),
                }
            }

            let total_fields = all_results.len() + failures.len();
            let strategy_matches = all_results.iter().filter(|r| r.strategies_match).count();
            let code_matches = all_results
                .iter()
                .filter(|r| r.from_proto_generation_matches && r.to_proto_generation_matches)
                .count();

            CompatibilityReport {
                total_fields,
                strategy_matches,
                code_matches,
                results: all_results,
                failures,
            }
        }
    }
}
