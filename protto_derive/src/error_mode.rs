use crate::expect_analysis::ExpectMode;
use crate::field_analysis::FieldProcessingContext;
use crate::field_info::RustFieldInfo;

/// Error handling mode that consolidates separate strategies for each error handling approach.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorMode {
    /// No special error handling - may panic on missing values
    None,

    /// Use .expect() with descriptive panic message
    Panic,

    /// Generate error handling code that returns Result
    Error,

    /// Use default value - None means Default::default(), Some(fn) means custom function
    Default(Option<String>),
}

impl ErrorMode {
    /// Create ErrorMode from existing field analysis
    pub fn from_field_context(ctx: &FieldProcessingContext, rust: &RustFieldInfo) -> Self {
        match rust.expect_mode {
            ExpectMode::Panic => Self::Panic,
            ExpectMode::Error => Self::Error,
            ExpectMode::None if rust.has_default || ctx.default_fn.is_some() => {
                Self::Default(ctx.default_fn.clone())
            }
            ExpectMode::None => Self::None,
        }
    }

    /// Check if this mode requires error handling infrastructure
    pub fn requires_error_handling(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// Check if this mode uses default values
    pub fn uses_default(&self) -> bool {
        matches!(self, Self::Default(_))
    }

    /// Check if this mode will panic on missing values
    pub fn will_panic(&self) -> bool {
        matches!(self, Self::Panic | Self::None)
    }

    /// Get the default function name if specified
    pub fn default_function(&self) -> Option<&str> {
        match self {
            Self::Default(Some(fn_name)) => Some(fn_name),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_mode_detection() {
        assert!(ErrorMode::Error.requires_error_handling());
        assert!(!ErrorMode::None.requires_error_handling());

        assert!(ErrorMode::Default(None).uses_default());
        assert!(ErrorMode::Default(Some("my_fn".to_string())).uses_default());
        assert!(!ErrorMode::Panic.uses_default());

        assert!(ErrorMode::Panic.will_panic());
        assert!(ErrorMode::None.will_panic());
        assert!(!ErrorMode::Error.will_panic());
    }

    #[test]
    fn test_default_function_extraction() {
        let mode_with_fn = ErrorMode::Default(Some("custom_default".to_string()));
        assert_eq!(mode_with_fn.default_function(), Some("custom_default"));

        let mode_without_fn = ErrorMode::Default(None);
        assert_eq!(mode_without_fn.default_function(), None);

        let panic_mode = ErrorMode::Panic;
        assert_eq!(panic_mode.default_function(), None);
    }
}

// show how existing strategies map to ErrorMode
impl ErrorMode {
    /// Map from old strategy to new error mode (for migration/testing)
    pub fn from_old_strategy(strategy: &crate::conversion::ConversionStrategy) -> Option<Self> {
        match strategy {
            crate::conversion::ConversionStrategy::UnwrapOptionalWithExpect => Some(Self::Panic),
            crate::conversion::ConversionStrategy::UnwrapOptionalWithError => Some(Self::Error),
            crate::conversion::ConversionStrategy::UnwrapOptionalWithDefault => {
                Some(Self::Default(None))
            }

            crate::conversion::ConversionStrategy::TransparentOptionalWithExpect => {
                Some(Self::Panic)
            }
            crate::conversion::ConversionStrategy::TransparentOptionalWithError => {
                Some(Self::Error)
            }
            crate::conversion::ConversionStrategy::TransparentOptionalWithDefault => {
                Some(Self::Default(None))
            }

            crate::conversion::ConversionStrategy::CollectVecWithDefault => {
                Some(Self::Default(None))
            }
            crate::conversion::ConversionStrategy::CollectVecWithError => Some(Self::Error),

            crate::conversion::ConversionStrategy::MapOptionWithDefault => {
                Some(Self::Default(None))
            }

            // Strategies that don't have explicit error modes
            _ => None,
        }
    }
}
