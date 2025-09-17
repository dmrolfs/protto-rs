use crate::debug::CallStackDebug;
use crate::error::mode::ErrorMode;
use crate::field::info::RustFieldInfo;

/// Consolidated custom function strategy that replaces separate strategies
/// for each combination of custom functions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CustomConversionStrategy {
    /// Only proto->rust conversion function provided
    FromFn(String),

    /// Only rust->proto conversion function provided
    IntoFn(String),

    /// Both directions have custom functions
    Bidirectional(String, String),
}

impl CustomConversionStrategy {
    /// Detect custom strategy from field analysis
    pub fn from_field_info(name: &syn::Ident, rust: &RustFieldInfo) -> Option<Self> {
        let _trace = CallStackDebug::with_context(
            "conversion::custom_strategy::CustomConversionStrategy",
            "from_field_info",
            name,
            &rust.field_name,
            &[
                ("from_proto_fn", &format!("{:?}", rust.from_proto_fn)),
                ("to_proto_fn", &format!("{:?}", rust.to_proto_fn)),
            ],
        );

        let strategy = match (&rust.from_proto_fn, &rust.to_proto_fn) {
            (Some(from_fn), Some(into_fn)) => {
                Some(Self::Bidirectional(from_fn.clone(), into_fn.clone()))
            }

            (Some(from_fn), None) => {
                Some(Self::FromFn(from_fn.clone()))
            },

            (None, Some(into_fn)) => Some(Self::IntoFn(into_fn.clone())),

            (None, None) => None,
        };

        _trace.checkpoint_data(
            "custom_conversion_strategy",
            &[("strategy", &format!("{:?}", strategy)),]
        );

        strategy
    }

    /// Get the proto->rust function name if available
    pub fn from_proto_fn(&self) -> Option<&str> {
        match self {
            Self::FromFn(fn_name) | Self::Bidirectional(fn_name, _) => Some(fn_name),
            Self::IntoFn(_) => None,
        }
    }

    /// Get the rust->proto function name if available
    pub fn to_proto_fn(&self) -> Option<&str> {
        match self {
            Self::IntoFn(fn_name) | Self::Bidirectional(_, fn_name) => Some(fn_name),
            Self::FromFn(_) => None,
        }
    }

    /// Check if proto->rust conversion is available
    pub fn has_from_proto_fn(&self) -> bool {
        matches!(self, Self::FromFn(_) | Self::Bidirectional(_, _))
    }

    /// Check if rust->proto conversion is available
    pub fn has_to_proto_fn(&self) -> bool {
        matches!(self, Self::IntoFn(_) | Self::Bidirectional(_, _))
    }

    /// Check if both directions are available
    pub fn is_bidirectional(&self) -> bool {
        matches!(self, Self::Bidirectional(_, _))
    }

    /// Validate that function paths are not empty
    pub fn validate(&self) -> Result<(), String> {
        let validate_path = |path: &str, direction: &str| -> Result<(), String> {
            if path.trim().is_empty() {
                return Err(format!(
                    "Custom {} function path cannot be empty",
                    direction
                ));
            }
            // Could add more validation here (valid identifier, etc.)
            Ok(())
        };

        match self {
            Self::FromFn(path) => {
                validate_path(path, "proto_to_rust")
            },
            Self::IntoFn(path) => validate_path(path, "rust_to_proto"),
            Self::Bidirectional(from_path, into_path) => {
                validate_path(from_path, "proto_to_rust")?;
                validate_path(into_path, "rust_to_proto")?;
                Ok(())
            }
        }
    }

    /// Determine if custom function needs error handling
    fn needs_error_handling(rust_field_info: &RustFieldInfo) -> bool {
        // If field has explicit error handling attributes
        if rust_field_info.expect_mode != crate::analysis::expect_analysis::ExpectMode::None {
            return true;
        }

        // Default behavior for custom functions with no explicit error attributes
        // The old system would apply UnwrapOptionalWithExpect for complex types with custom functions
        !rust_field_info.is_option && !rust_field_info.is_primitive && rust_field_info.is_custom
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_rust_field_with_fns(
        from_proto_fn: Option<String>,
        to_proto_fn: Option<String>,
    ) -> RustFieldInfo {
        let field_name: syn::Ident = syn::parse_str("test_field").unwrap();
        let field_type: syn::Type = syn::parse_str("String").unwrap();

        RustFieldInfo {
            field_name,
            field_type,
            is_option: false,
            is_vec: false,
            is_primitive: true,
            is_custom: false,
            is_enum: false,
            has_transparent: false,
            has_default: false,
            expect_mode: crate::analysis::expect_analysis::ExpectMode::None,
            has_proto_ignore: false,
            from_proto_fn,
            to_proto_fn,
        }
    }

    #[test]
    fn test_custom_strategy_detection() {
        let field_name: syn::Ident = syn::parse_str("test_field").unwrap();
        // Test bidirectional
        let rust =
            mock_rust_field_with_fns(Some("from_proto".to_string()), Some("to_proto".to_string()));
        let strategy = CustomConversionStrategy::from_field_info(&field_name, &rust);
        assert_eq!(
            strategy,
            Some(CustomConversionStrategy::Bidirectional(
                "from_proto".to_string(),
                "to_proto".to_string()
            ))
        );

        // Test proto->rust only
        let rust = mock_rust_field_with_fns(Some("from_proto".to_string()), None);
        let strategy = CustomConversionStrategy::from_field_info(&field_name, &rust);
        assert_eq!(
            strategy,
            Some(CustomConversionStrategy::FromFn("from_proto".to_string()))
        );

        // Test rust->proto only
        let rust = mock_rust_field_with_fns(None, Some("to_proto".to_string()));
        let strategy = CustomConversionStrategy::from_field_info(&field_name, &rust);
        assert_eq!(
            strategy,
            Some(CustomConversionStrategy::IntoFn("to_proto".to_string()))
        );

        // Test no custom functions
        let rust = mock_rust_field_with_fns(None, None);
        let strategy = CustomConversionStrategy::from_field_info(&field_name, &rust);
        assert_eq!(strategy, None);
    }

    #[test]
    fn test_function_name_extraction() {
        let bidirectional =
            CustomConversionStrategy::Bidirectional("from_fn".to_string(), "into_fn".to_string());
        assert_eq!(bidirectional.from_proto_fn(), Some("from_fn"));
        assert_eq!(bidirectional.to_proto_fn(), Some("into_fn"));
        assert!(bidirectional.has_from_proto_fn());
        assert!(bidirectional.has_to_proto_fn());
        assert!(bidirectional.is_bidirectional());

        let from_only = CustomConversionStrategy::FromFn("from_fn".to_string());
        assert_eq!(from_only.from_proto_fn(), Some("from_fn"));
        assert_eq!(from_only.to_proto_fn(), None);
        assert!(from_only.has_from_proto_fn());
        assert!(!from_only.has_to_proto_fn());
        assert!(!from_only.is_bidirectional());

        let into_only = CustomConversionStrategy::IntoFn("into_fn".to_string());
        assert_eq!(into_only.from_proto_fn(), None);
        assert_eq!(into_only.to_proto_fn(), Some("into_fn"));
        assert!(!into_only.has_from_proto_fn());
        assert!(into_only.has_to_proto_fn());
        assert!(!into_only.is_bidirectional());
    }

    #[test]
    fn test_validation() {
        let valid = CustomConversionStrategy::FromFn("valid_function_name".to_string());
        assert!(valid.validate().is_ok());

        let empty = CustomConversionStrategy::FromFn("".to_string());
        assert!(empty.validate().is_err());

        let whitespace = CustomConversionStrategy::IntoFn("   ".to_string());
        assert!(whitespace.validate().is_err());

        let bidirectional_valid =
            CustomConversionStrategy::Bidirectional("from_fn".to_string(), "into_fn".to_string());
        assert!(bidirectional_valid.validate().is_ok());

        let bidirectional_invalid =
            CustomConversionStrategy::Bidirectional("from_fn".to_string(), "".to_string());
        assert!(bidirectional_invalid.validate().is_err());
    }
}

// Mapping from old strategies to new custom strategy (for migration/testing)
impl CustomConversionStrategy {
    /// Map from old strategy to new custom strategy (for migration/testing)
    pub fn from_old_strategy(strategy: &crate::conversion::ConversionStrategy) -> Option<Self> {
        match strategy {
            crate::conversion::ConversionStrategy::DeriveProtoToRust(path) => {
                Some(Self::FromFn(path.clone()))
            }
            crate::conversion::ConversionStrategy::DeriveRustToProto(path) => {
                Some(Self::IntoFn(path.clone()))
            }
            crate::conversion::ConversionStrategy::DeriveBidirectional(from_path, into_path) => {
                Some(Self::Bidirectional(from_path.clone(), into_path.clone()))
            }
            _ => None,
        }
    }

    /// Convert back to old strategy format (for compatibility during migration)
    pub fn to_old_strategy(&self) -> crate::conversion::ConversionStrategy {
        match self {
            Self::FromFn(path) => {
                crate::conversion::ConversionStrategy::DeriveProtoToRust(path.clone())
            }
            Self::IntoFn(path) => {
                crate::conversion::ConversionStrategy::DeriveRustToProto(path.clone())
            }
            Self::Bidirectional(from_path, into_path) => {
                crate::conversion::ConversionStrategy::DeriveBidirectional(
                    from_path.clone(),
                    into_path.clone(),
                )
            }
        }
    }
}
