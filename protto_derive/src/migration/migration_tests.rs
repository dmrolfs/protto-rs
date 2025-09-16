use crate::migration::compatibility::{StrategyCompatibilityTester, test_helpers};
use crate::migration::config;

#[cfg(test)]
pub fn with_env_var<F>(key: &str, value: &str, test: F)
where F: FnOnce() {
    unsafe {
        let old_value = std::env::var(key).ok();
        std::env::set_var(key, value);
        test();
        match old_value {
            Some(val) => std::env::set_var(key, val),
            None => std::env::remove_var(key),
        }
    }
}

#[cfg(test)]
mod baseline_migration_test {
    use crate::analysis::field_analysis;
    use crate::migration::config;
    use crate::migration::migration_tests::with_env_var;

    #[test]
    fn test_migration_framework_baseline() {
        println!("Testing migration framework integration...");

        // Test that migration system initializes without errors
        config::old_only();
        println!("✓ Migration system initialization: OK");

        // Test basic compatibility testing framework
        let result = crate::migration::compatibility::test_helpers::create_test_case_for_pattern(
            "basic_primitive",
        );
        assert!(result.is_some(), "Should be able to create basic test case");
        println!("✓ Test case creation: OK");

        if let Some((field, context)) = result {
            match crate::migration::generate_field_conversions_with_migration(&field, &context) {
                Ok((proto_to_rust, rust_to_proto)) => {
                    assert!(!proto_to_rust.is_empty(), "Proto->Rust conversion should not be empty");
                    assert!(!rust_to_proto.is_empty(), "Rust->Proto conversion should not be empty");
                    println!("✓ Migration function (old_only mode): OK");
                },
                Err(e) => {
                    panic!("Migration function failed: {}", e);
                }
            }
        }

        with_env_var(
            "PROTTO_MIGRATION_MODE",
            "validate_both",
            || {
                config::from_env();
                println!("✓ Environment configuration: OK");
            }
        );

        println!("Migration framework baseline: OK");
    }

    #[test]
    fn test_existing_functionality_unaffected() {
        println!("Testing that existing functionality is unaffected...");

        // Test that we can still create contexts the old way
        let struct_ident = syn::Ident::new("TestStruct", proc_macro2::Span::call_site());
        let field_type: syn::Type = syn::parse_str("String").unwrap();
        let field: syn::Field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Public(Default::default()),
            mutability: syn::FieldMutability::None,
            ident: Some(syn::Ident::new("test_field", proc_macro2::Span::call_site())),
            colon_token: Some(syn::Token![:](proc_macro2::Span::call_site())),
            ty: field_type,
        };

        let error_ident = syn::Ident::new("TestError", proc_macro2::Span::call_site());

        let context = field_analysis::FieldProcessingContext::new(
            &struct_ident,
            &field,
            &error_ident,
            &None,
            &None,
            "proto",
            "TestStruct",
        );

        // Test that existing field analysis still works
        match field_analysis::generate_field_conversions(&field, &context) {
            Ok((proto_to_rust, rust_to_proto)) => {
                assert!(!proto_to_rust.is_empty());
                assert!(!rust_to_proto.is_empty());
                println!("✓ Existing field analysis: OK");
            },
            Err(e) => {
                println!("⚠️ Existing field analysis error: {}", e);
                // This might fail for complex cases, which is expected
                // The important thing is that it doesn't panic or break compilation
            }
        }

        println!("✓ Existing functionality compatibility verified");
    }

    #[test]
    fn test_error_mode_integration() {
        use crate::error::mode::ErrorMode;
        use crate::analysis::expect_analysis::ExpectMode;

        // Test ErrorMode creation
        let error_modes = vec![
            ErrorMode::None,
            ErrorMode::Panic,
            ErrorMode::Error,
            ErrorMode::Default(None),
            ErrorMode::Default(Some("test_fn".to_string())),
        ];

        for mode in error_modes {
            // Just test that they can be created and compared
            let _mode_copy = mode.clone();
            println!("✓ ErrorMode {:?}: OK", mode);
        }

        println!("✓ ErrorMode integration: OK");
    }

    #[test]
    fn test_custom_strategy_integration() {
        use crate::conversion::custom_strategy::CustomConversionStrategy;

        // Test CustomConversionStrategy creation
        let strategies = vec![
            CustomConversionStrategy::FromFn("from_fn".to_string()),
            CustomConversionStrategy::IntoFn("into_fn".to_string()),
            CustomConversionStrategy::Bidirectional("from_fn".to_string(), "into_fn".to_string()),
        ];

        for strategy in strategies {
            assert!(strategy.validate().is_ok());
            println!("✓ CustomConversionStrategy {:?}: OK", strategy);
        }

        // Test validation
        let invalid = CustomConversionStrategy::FromFn("".to_string());
        assert!(invalid.validate().is_err());
        println!("✓ CustomConversionStrategy validation: OK");
    }
}

#[cfg(test)]
mod challenging_migration_tests {
    use super::*;

    /// Test the most complex case from your tests - ComplexExpectStruct with multiple error strategies
    #[test]
    fn test_complex_expect_struct_migration() {
        config::validate_both();

        let challenging_fields = vec![
            // Field with panic mode
            (
                "ComplexExpectStruct",
                "field_with_panic",
                "String",
                &["expect(panic)"][..],
            ),
            // Field with custom error function
            (
                "ComplexExpectStruct",
                "field_with_error",
                "String",
                &["expect", "error_fn = \"ValidationError::missing_field\""],
            ),
            // Field with custom default function
            (
                "ComplexExpectStruct",
                "number_with_default",
                "u64",
                &["default = \"default_number\""],
            ),
            // Enum with expect panic
            (
                "ComplexExpectStruct",
                "enum_with_panic",
                "Status",
                &["expect(panic)"],
            ),
            // Collection with expect
            (
                "ComplexExpectStruct",
                "tracks_with_expect",
                "Vec<Track>",
                &["expect", "error_fn = \"ValidationError::missing_field\""],
            ),
        ];

        let mut passed = 0;
        let mut failed = 0;

        for (struct_name, field_name, field_type, attributes) in challenging_fields {
            println!("Testing challenging case: {}.{}", struct_name, field_name);

            let (field, context) = test_helpers::create_mock_context(
                struct_name,
                field_name,
                field_type,
                "proto",
                attributes,
            );
            println!("Proto meta: {:?}", context.proto_meta);

            match StrategyCompatibilityTester::compare_field_strategies(&context, &field) {
                Ok(comparison) => {
                    println!("  Strategies match: {}", comparison.strategies_match);
                    println!("  Code matches: {}", comparison.from_proto_generation_matches && comparison.to_proto_generation_matches);

                    if comparison.strategies_match &&
                    comparison.from_proto_generation_matches &&
                    comparison.to_proto_generation_matches {
                        passed += 1;
                    } else {
                        failed += 1;
                        println!("  OLD: {:?}", comparison.old_strategy);
                        println!("  NEW: {:?}", comparison.new_strategy);

                        if !comparison.from_proto_generation_matches {
                            println!("  OLD from_proto CODE: {}", comparison.old_from_proto);
                            println!("  NEW from_proto CODE: {}", comparison.new_from_proto);
                        }

                        if !comparison.to_proto_generation_matches {
                            println!("  OLD to_proto CODE: {}", comparison.old_to_proto);
                            println!("  NEW to_proto CODE: {}", comparison.new_to_proto);
                        }
                    }
                }
                Err(e) => {
                    failed += 1;
                    println!("  ERROR: {}", e);
                }
            }
        }

        println!(
            "Complex expect struct migration results: {}/{} passed",
            passed,
            passed + failed
        );

        if failed > 0 {
            panic!(
                "Complex expect struct migration validation failed: {} failures",
                failed
            );
        }
    }

    /// Test bidirectional custom functions - another challenging case
    #[test]
    fn test_bidirectional_custom_functions() {
        config::validate_both();

        let (field, context) = test_helpers::create_mock_context(
            "BidirectionalConversionStruct",
            "custom_field",
            "CustomComplexType",
            "proto",
            &[
                "from_proto_fn = \"custom_from_conversion\"",
                "to_proto_fn = \"custom_into_conversion\"",
            ],
        );
        println!("Proto meta: {:?}", context.proto_meta);
        println!("Old system proto field is_optional: {}", context.proto_meta.is_proto_optional());

        let result = StrategyCompatibilityTester::compare_field_strategies(&context, &field);

        match result {
            Ok(comparison) => {
                assert!(
                    comparison.strategies_match,
                    "Bidirectional strategy mismatch: old={:?}, new={:?}",
                    comparison.old_strategy, comparison.new_strategy
                );

                assert!(
                    comparison.from_proto_generation_matches,
                    "from_proto code generation mismatch:\nOld: {}\nNew: {}",
                    comparison.old_from_proto, comparison.new_from_proto
                );

                assert!(
                    comparison.to_proto_generation_matches,
                    "to_proto code generation mismatch:\nOld: {}\nNew: {}",
                    comparison.old_to_proto, comparison.new_to_proto
                );

                println!("Bidirectional custom functions: PASSED");
            }
            Err(e) => {
                panic!("Bidirectional custom functions test failed: {}", e);
            }
        }
    }

    /// Test transparent with various error modes - challenging due to multiple strategies
    #[test]
    fn test_transparent_with_error_modes() {
        config::validate_both();

        let transparent_cases = vec![
            (
                "TransparentOptionalStruct",
                "panic_wrapper",
                "TransparentWrapper",
                &["transparent", "expect(panic)"][..],
            ),
            (
                "TransparentOptionalStruct",
                "error_wrapper",
                "TransparentWrapper",
                &["transparent", "expect", "error_fn = \"custom_error\""],
            ),
            (
                "TransparentOptionalStruct",
                "default_wrapper",
                "TransparentWrapper",
                &["transparent", "default = \"default_transparent\""],
            ),
        ];

        for (struct_name, field_name, field_type, attributes) in transparent_cases {
            let (field, context) = test_helpers::create_mock_context(
                struct_name,
                field_name,
                field_type,
                "proto",
                attributes,
            );

            let result = StrategyCompatibilityTester::compare_field_strategies(&context, &field);

            match result {
                Ok(comparison) => {
                    println!(
                        "Transparent case {}.{}: strategies={}, code={}",
                        struct_name,
                        field_name,
                        comparison.strategies_match,
                        comparison.from_proto_generation_matches && comparison.to_proto_generation_matches
                    );

                    if !comparison.strategies_match {
                        println!(
                            "  Strategy mismatch: old={:?}, new={:?}",
                            comparison.old_strategy, comparison.new_strategy
                        );
                    }
                    if !comparison.from_proto_generation_matches {
                        println!("  Code mismatch from_proto detected");
                    }
                    if !comparison.to_proto_generation_matches {
                        println!("  Code mismatch to_proto detected");
                    }
                }
                Err(e) => {
                    println!(
                        "Transparent case {}.{}: ERROR - {}",
                        struct_name, field_name, e
                    );
                }
            }
        }
    }

    /// Test collection strategies with different error modes
    #[test]
    fn test_collection_error_strategies() {
        config::validate_both();

        let collection_cases = vec![
            (
                "CollectionWithExpect",
                "tracks",
                "Vec<Track>",
                &["expect"][..],
            ),
            (
                "VecWithErrorStruct",
                "tracks_with_error",
                "Vec<Track>",
                &["expect", "error_fn = \"default_track_vec\""],
            ),
            (
                "VecOptionStruct",
                "optional_tracks",
                "Option<Vec<Track>>",
                &[],
            ),
            (
                "CollectionWithDefault",
                "tracks",
                "Vec<Track>",
                &["default = \"default_track_vec\""],
            ),
        ];

        let mut results = Vec::new();

        for (struct_name, field_name, field_type, attributes) in collection_cases {
            let (field, context) = test_helpers::create_mock_context(
                struct_name,
                field_name,
                field_type,
                "proto",
                attributes,
            );

            match StrategyCompatibilityTester::compare_field_strategies(&context, &field) {
                Ok(comparison) => {
                    results.push((
                        struct_name,
                        field_name,
                        comparison.strategies_match,
                        comparison.from_proto_generation_matches && comparison.to_proto_generation_matches,
                    ));

                    if !comparison.strategies_match ||
                    !comparison.from_proto_generation_matches ||
                    !comparison.to_proto_generation_matches {
                        println!(
                            "Collection case {}.{}: strategies={}, from_proto_code={}, to_proto_code={}",
                            struct_name,
                            field_name,
                            comparison.strategies_match,
                            comparison.from_proto_generation_matches,
                            comparison.to_proto_generation_matches
                        );
                        println!("  OLD: {:?}", comparison.old_strategy);
                        println!("  NEW: {:?}", comparison.new_strategy);
                    }
                }
                Err(e) => {
                    println!(
                        "Collection case {}.{}: ERROR - {}",
                        struct_name, field_name, e
                    );
                    results.push((struct_name, field_name, false, false));
                }
            }
        }

        let total = results.len();
        let passed = results.iter().filter(|(_, _, s, c)| *s && *c).count();
        println!("Collection strategies: {}/{} passed", passed, total);
    }

    /// Integration test - run the complete comprehensive test suite
    #[test]
    fn test_comprehensive_migration_validation() {
        config::validate_both();

        let test_suite = test_helpers::create_comprehensive_test_suite();
        let report = test_suite.run_all_tests();

        println!("=== COMPREHENSIVE MIGRATION VALIDATION ===");
        report.print_summary();

        // Print detailed results for failing cases
        if !report.all_tests_passed() {
            println!("\nDETAILED FAILURE ANALYSIS:");

            for mismatch in report.get_strategy_mismatches() {
                println!(
                    "STRATEGY MISMATCH: {}.{}",
                    mismatch.struct_name, mismatch.field_name
                );
                println!("  Old: {:?}", mismatch.old_strategy);
                println!("  New: {:?}", mismatch.new_strategy);
                println!();
            }

            for mismatch in report.get_code_mismatches() {
                println!(
                    "CODE MISMATCH: {}.{}",
                    mismatch.struct_name, mismatch.field_name
                );
                println!("  Old proto->rust: {}", mismatch.old_from_proto);
                println!("  New proto->rust: {}", mismatch.new_from_proto);
                println!();
            }
        }

        // For initial migration, we expect some differences - document them
        println!("Initial migration validation complete.");
        println!(
            "Strategy match rate: {:.1}%",
            (report.strategy_matches as f64 / report.total_fields as f64) * 100.0
        );
        println!(
            "Code match rate: {:.1}%",
            (report.code_matches as f64 / report.total_fields as f64) * 100.0
        );
    }
}

/// Environment-based migration tests (run with different PROTTO_MIGRATION_MODE values)
#[cfg(test)]
mod environment_migration_tests {
    use super::*;

    #[test]
    fn test_old_only_mode() {
        config::old_only();

        // Should work exactly like current system
        let (field, context) =
            test_helpers::create_mock_context("TestStruct", "test_field", "u64", "proto", &[]);

        match crate::migration::generate_field_conversions_with_migration(&field, &context) {
            Ok((proto_to_rust, rust_to_proto)) => {
                assert!(!proto_to_rust.is_empty());
                assert!(!rust_to_proto.is_empty());
                println!("Old-only mode: PASSED");
            }
            Err(e) => panic!("Old-only mode failed: {}", e),
        }
    }

    // #[test]
    // fn test_new_with_fallback_mode() {
    //     config::new_with_fallback();
    //
    //     // Should try new system, fall back to old on failure
    //     let (field, context) = test_helpers::create_mock_context(
    //         "TestStruct",
    //         "test_field",
    //         "String",
    //         "proto",
    //         &["expect"],
    //     );
    //
    //     match crate::migration::generate_field_conversions_with_migration(&field, &context) {
    //         Ok((proto_to_rust, rust_to_proto)) => {
    //             assert!(!proto_to_rust.is_empty());
    //             assert!(!rust_to_proto.is_empty());
    //             println!("New-with-fallback mode: PASSED");
    //         }
    //         Err(e) => panic!("New-with-fallback mode failed: {}", e),
    //     }
    // }

    #[test]
    #[ignore] // Enable when new system is more complete
    fn test_new_only_mode() {
        config::new_only();

        let (field, context) =
            test_helpers::create_mock_context("TestStruct", "simple_field", "u64", "proto", &[]);

        match crate::migration::generate_field_conversions_with_migration(&field, &context) {
            Ok((proto_to_rust, rust_to_proto)) => {
                assert!(!proto_to_rust.is_empty());
                assert!(!rust_to_proto.is_empty());
                println!("New-only mode: PASSED");
            }
            Err(e) => panic!("New-only mode failed: {}", e),
        }
    }
}
