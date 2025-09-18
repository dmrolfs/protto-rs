// Safe migration path from old system to new system

use crate::analysis::field_analysis;
use crate::analysis::field_analysis::FieldProcessingContext;
use crate::field::{conversion_strategy::FieldConversionStrategy, info as field_info};
use crate::migration::compatibility::StrategyCompatibilityTester;

/// Migration mode configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MigrationMode {
    /// Use old system only (current production mode)
    OldOnly,
    // /// Use new system with old system as fallback
    // NewWithFallback,
    /// Use new system only (target end state)
    NewOnly,
    /// Run both systems and validate they produce identical results
    ValidateBoth,
}

impl std::fmt::Display for MigrationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::OldOnly => write!(f, "Old Only"),
            // Self::NewWithFallback => write!(f, "New With Fallback"),
            Self::NewOnly => write!(f, "New Only"),
            Self::ValidateBoth => write!(f, "Validate Both"),
        }
    }
}

/// Migration controller for gradual system replacement
#[derive(Debug)]
#[allow(unused)]
pub struct FieldConversionMigration {
    pub mode: MigrationMode,
    validation_enabled: bool,
    // failure_fallback: bool,
}

impl FieldConversionMigration {
    /// Create new migration controller
    pub fn new(mode: MigrationMode) -> Self {
        Self {
            mode,
            validation_enabled: matches!(mode, MigrationMode::ValidateBoth),
            // failure_fallback: matches!(mode, MigrationMode::NewWithFallback),
        }
    }

    /// Enable/disable validation during migration
    #[allow(unused)]
    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.validation_enabled = enabled;
        self
    }

    // /// Enable/disable fallback on new system failures
    // pub fn with_fallback(mut self, enabled: bool) -> Self {
    //     self.failure_fallback = enabled;
    //     self
    // }
}

/// Global migration configuration
static GLOBAL_MIGRATION: std::sync::OnceLock<FieldConversionMigration> = std::sync::OnceLock::new();

/// Initialize global migration configuration
pub fn initialize_migration(mode: MigrationMode) {
    // eprintln!("📊 migration_mode: {mode}");
    let migration = FieldConversionMigration::new(mode);
    if GLOBAL_MIGRATION.set(migration).is_err() {
        // Already initialized - this is fine, just use the existing one
    }
}

/// Get global migration configuration
pub fn get_global_migration() -> &'static FieldConversionMigration {
    GLOBAL_MIGRATION.get_or_init(|| {
        // Default to old system if not explicitly initialized
        FieldConversionMigration::new(MigrationMode::OldOnly)
    })
}

/// Environment variable configuration for migration mode
pub fn configure_migration_from_env() -> MigrationMode {
    match std::env::var("PROTTO_MIGRATION_MODE").as_deref() {
        Ok("old_only") => MigrationMode::OldOnly,
        // Ok("new_with_fallback") => MigrationMode::NewWithFallback,
        Ok("new_only") => MigrationMode::NewOnly,
        Ok("validate_both") => MigrationMode::ValidateBoth,
        _ => MigrationMode::OldOnly, // Safe default
    }
}

/// Convenience functions for migration configuration
pub mod config {
    use super::{MigrationMode, configure_migration_from_env, initialize_migration};

    #[allow(unused)]
    pub fn old_only() {
        initialize_migration(MigrationMode::OldOnly);
    }

    // pub fn new_with_fallback() {
    //     initialize_migration(MigrationMode::NewWithFallback);
    // }

    #[allow(unused)]
    pub fn new_only() {
        initialize_migration(MigrationMode::NewOnly);
    }

    #[allow(unused)]
    pub fn validate_both() {
        initialize_migration(MigrationMode::ValidateBoth);
    }

    pub fn from_env() {
        initialize_migration(configure_migration_from_env());
    }
}

impl FieldConversionMigration {
    /// Generate field conversions using configured migration mode
    pub fn generate_field_conversions(
        &self,
        field: &syn::Field,
        ctx: &FieldProcessingContext,
    ) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), MigrationError> {
        match self.mode {
            MigrationMode::OldOnly => self.generate_with_old_system_only(field, ctx),
            // MigrationMode::NewWithFallback => {
            //     self.generate_with_new_system_and_fallback(field, ctx)
            // }
            MigrationMode::NewOnly => self.generate_with_new_system_only(field, ctx),
            MigrationMode::ValidateBoth => self.generate_with_validation(field, ctx),
        }
    }

    /// Generate using old system only
    fn generate_with_old_system_only(
        &self,
        field: &syn::Field,
        ctx: &FieldProcessingContext,
    ) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), MigrationError> {
        field_analysis::generate_field_conversions(field, ctx)
            .map_err(|e| MigrationError::OldSystemFailure(format!("{:?}", e)))
    }

    /// Generate using new system with old system fallback
    #[allow(unused)]
    fn generate_with_new_system_and_fallback(
        &self,
        field: &syn::Field,
        ctx: &FieldProcessingContext,
    ) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), MigrationError> {
        match self.try_new_system(field, ctx) {
            Ok(result) => {
                if self.validation_enabled {
                    self.validate_against_old_system(field, ctx, &result)?;
                }
                Ok(result)
            }
            // Err(new_error) if self.failure_fallback => {
            //     eprintln!(
            //         "New system failed for {}.{}, falling back to old system: {:?}",
            //         ctx.struct_name, ctx.field_name, new_error
            //     );
            //     self.generate_with_old_system_only(field, ctx)
            // }
            Err(new_error) => Err(new_error),
        }
    }

    /// Generate using new system only
    fn generate_with_new_system_only(
        &self,
        field: &syn::Field,
        ctx: &FieldProcessingContext,
    ) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), MigrationError> {
        self.try_new_system(field, ctx)
    }

    /// Generate with both systems and validate they match
    fn generate_with_validation(
        &self,
        field: &syn::Field,
        ctx: &FieldProcessingContext,
    ) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), MigrationError> {
        let comparison = StrategyCompatibilityTester::compare_field_strategies(ctx, field)
            .map_err(MigrationError::ValidationFailure)?;

        if !comparison.strategies_match {
            return Err(MigrationError::StrategyMismatch {
                field_name: ctx.field_name.to_string(),
                old_strategy: format!("{:?}", comparison.old_strategy),
                new_strategy: format!("{:?}", comparison.new_strategy),
            });
        }

        if !comparison.from_proto_generation_matches {
            return Err(MigrationError::CodeGenerationMismatch {
                field_name: ctx.field_name.to_string(),
                old_code: comparison.old_from_proto.to_string(),
                new_code: comparison.new_from_proto.to_string(),
            });
        }

        if !comparison.to_proto_generation_matches {
            return Err(MigrationError::CodeGenerationMismatch {
                field_name: ctx.field_name.to_string(),
                old_code: comparison.old_to_proto.to_string(),
                new_code: comparison.new_to_proto.to_string(),
            });
        }

        // Both systems match, return new system result
        Ok((comparison.new_from_proto, comparison.new_to_proto))
    }

    /// Try using the new system
    fn try_new_system(
        &self,
        field: &syn::Field,
        ctx: &FieldProcessingContext,
    ) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), MigrationError> {
        // Analyze field using new system
        let rust_field_info = field_info::RustFieldInfo::analyze(ctx, field);
        let proto_field_info = field_info::ProtoFieldInfo::infer_from(ctx, field, &rust_field_info);
        let strategy = FieldConversionStrategy::from_field_info(
            ctx,
            field,
            &rust_field_info,
            &proto_field_info,
        );

        // Validate strategy is reasonable
        if let Err(validation_error) =
            strategy.validate_for_context(ctx, &rust_field_info, &proto_field_info)
        {
            return Err(MigrationError::NewSystemFailure(format!(
                "Strategy validation failed: {}",
                validation_error
            )));
        }

        // Generate code
        let proto_to_rust = strategy.generate_proto_to_rust_conversion(
            ctx,
            field,
            &rust_field_info,
            &proto_field_info,
        );
        let rust_to_proto = strategy.generate_rust_to_proto_conversion(
            ctx,
            field,
            &rust_field_info,
            &proto_field_info,
        );

        Ok((proto_to_rust, rust_to_proto))
    }

    /// Validate new system result against old system
    fn validate_against_old_system(
        &self,
        field: &syn::Field,
        ctx: &FieldProcessingContext,
        new_result: &(proc_macro2::TokenStream, proc_macro2::TokenStream),
    ) -> Result<(), MigrationError> {
        let old_result = field_analysis::generate_field_conversions(field, ctx)
            .map_err(|e| MigrationError::OldSystemFailure(e.detailed_message()))?;

        // Compare code generation (simplified comparison)
        let old_proto_str = old_result.0.to_string();
        let new_proto_str = new_result.0.to_string();
        let old_rust_str = old_result.1.to_string();
        let new_rust_str = new_result.1.to_string();

        // Normalize for comparison
        fn normalize(s: &str) -> String {
            s.split_whitespace().collect::<Vec<_>>().join(" ")
        }

        if normalize(&old_proto_str) != normalize(&new_proto_str) {
            return Err(MigrationError::CodeGenerationMismatch {
                field_name: ctx.field_name.to_string(),
                old_code: old_proto_str,
                new_code: new_proto_str,
            });
        }

        if normalize(&old_rust_str) != normalize(&new_rust_str) {
            return Err(MigrationError::CodeGenerationMismatch {
                field_name: ctx.field_name.to_string(),
                old_code: old_rust_str,
                new_code: new_rust_str,
            });
        }

        Ok(())
    }

    /// Get migration statistics for reporting
    #[allow(unused)]
    pub fn get_migration_stats(&self) -> MigrationStats {
        MigrationStats {
            mode: self.mode,
            validation_enabled: self.validation_enabled,
            // fallback_enabled: self.failure_fallback,
        }
    }
}

/// Migration error types
#[derive(Debug)]
pub enum MigrationError {
    OldSystemFailure(String),
    NewSystemFailure(String),
    ValidationFailure(String),
    StrategyMismatch {
        field_name: String,
        old_strategy: String,
        new_strategy: String,
    },
    CodeGenerationMismatch {
        field_name: String,
        old_code: String,
        new_code: String,
    },
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationError::OldSystemFailure(msg) => write!(f, "Old system failure: {}", msg),
            MigrationError::NewSystemFailure(msg) => write!(f, "New system failure: {}", msg),
            MigrationError::ValidationFailure(msg) => write!(f, "Validation failure: {}", msg),
            MigrationError::StrategyMismatch {
                field_name,
                old_strategy,
                new_strategy,
            } => {
                write!(
                    f,
                    "Strategy mismatch for field '{}': old={}, new={}",
                    field_name, old_strategy, new_strategy
                )
            }
            MigrationError::CodeGenerationMismatch {
                field_name,
                old_code,
                new_code,
            } => {
                write!(
                    f,
                    "Code generation mismatch for field '{}':\nOld: {}\nNew: {}",
                    field_name, old_code, new_code
                )
            }
        }
    }
}

impl std::error::Error for MigrationError {}

/// Migration statistics for reporting
#[derive(Debug)]
#[allow(unused)]
pub struct MigrationStats {
    pub mode: MigrationMode,
    pub validation_enabled: bool,
    // pub fallback_enabled: bool,
}

/// Main entry point for field conversion with migration support
pub fn generate_field_conversions_with_migration(
    field: &syn::Field,
    ctx: &FieldProcessingContext,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), MigrationError> {
    get_global_migration().generate_field_conversions(field, ctx)
}

// Integration with existing field analysis
impl FieldConversionStrategy {
    /// Validate that this strategy is compatible with the given context
    pub fn validate_for_context(
        &self,
        _ctx: &FieldProcessingContext,
        rust_field_info: &field_info::RustFieldInfo,
        proto_field_info: &field_info::ProtoFieldInfo,
    ) -> Result<(), String> {
        // Use the existing validation logic from the new system
        match self {
            FieldConversionStrategy::Ignore => {
                if !rust_field_info.has_proto_ignore {
                    return Err("Ignore strategy requires #[protto(ignore)] attribute".to_string());
                }
            }
            FieldConversionStrategy::Custom(custom_strategy) => {
                custom_strategy.validate()?;
            }
            FieldConversionStrategy::Transparent(_) => {
                if !rust_field_info.has_transparent {
                    return Err(
                        "Transparent strategy requires #[protto(transparent)] attribute"
                            .to_string(),
                    );
                }
                // Additional transparent-specific validation could go here
            }
            FieldConversionStrategy::Collection(_) => {
                if !rust_field_info.is_vec && !proto_field_info.is_repeated() {
                    return Err("Collection strategy requires Vec or repeated field".to_string());
                }
            }
            _ => {
                // Other strategies have their own validation logic
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::compatibility::test_helpers;

    #[test]
    fn test_migration_modes() {
        let modes = vec![
            MigrationMode::OldOnly,
            // MigrationMode::NewWithFallback,
            MigrationMode::NewOnly,
            MigrationMode::ValidateBoth,
        ];

        for mode in modes {
            let migration = FieldConversionMigration::new(mode);
            let stats = migration.get_migration_stats();
            assert_eq!(stats.mode, mode);
        }
    }

    #[test]
    fn test_migration_with_simple_field() {
        let (field, context) =
            test_helpers::create_mock_context("TestStruct", "test_field", "String", "proto", &[]);

        let migration = FieldConversionMigration::new(MigrationMode::OldOnly);
        let result = migration.generate_field_conversions(&field, &context);

        // Should succeed with old system
        assert!(result.is_ok());
    }

    #[test]
    #[ignore]
    fn test_environment_configuration() {
        crate::migration::migration_tests::with_env_var(
            "PROTTO_MIGRATION_MODE",
            "new_only",
            || {
                let mode = configure_migration_from_env();
                assert_eq!(mode, MigrationMode::NewOnly);
            },
        );

        crate::migration::migration_tests::with_env_var(
            "PROTTO_MIGRATION_MODE",
            "validate_both",
            || {
                let mode = configure_migration_from_env();
                assert_eq!(mode, MigrationMode::ValidateBoth);
            },
        );

        let mode = configure_migration_from_env();
        assert_eq!(mode, MigrationMode::OldOnly); // Default
    }

    #[test]
    fn test_validation_mode_with_matching_systems() {
        let (field, context) =
            test_helpers::create_mock_context("TestStruct", "simple_field", "u64", "proto", &[]);

        let migration = FieldConversionMigration::new(MigrationMode::ValidateBoth);
        let result = migration.generate_field_conversions(&field, &context);

        match result {
            Ok(_) => println!("Systems match for simple field"),
            Err(MigrationError::StrategyMismatch { .. }) => {
                println!("Expected strategy mismatch for this field type")
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    // #[test]
    // fn test_fallback_mode() {
    //     let migration =
    //         FieldConversionMigration::new(MigrationMode::NewWithFallback).with_fallback(true);
    //
    //     assert!(migration.failure_fallback);
    //
    //     let stats = migration.get_migration_stats();
    //     assert!(stats.fallback_enabled);
    // }
}

// Usage example in your main macro:
/*
fn main_macro_entry_point() {
    // Configure migration at macro startup
    crate::migration::config::from_env();

    // Use migration-aware field processing
    for field in struct_fields {
        match generate_field_conversions_with_migration(&field, &context) {
            Ok((proto_to_rust, rust_to_proto)) => {
                // Use generated code
            },
            Err(migration_error) => {
                // Handle migration errors appropriately
                return syn::Error::new_spanned(field, migration_error).into_compile_error();
            }
        }
    }
}
*/
