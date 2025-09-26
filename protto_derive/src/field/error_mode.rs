use crate::analysis::expect_analysis::ExpectMode;
use crate::field::FieldProcessingContext;
use crate::field::info::RustFieldInfo;

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
        // Priority 1: Default takes precedence over everything (including error_type)
        if rust.has_default || ctx.default_fn.is_some() {
            return Self::Default(ctx.default_fn.clone());
        }

        // Priority 2: When error_type is specified, it overrides panic behavior
        if ctx.struct_level_error_type.is_some() {
            // Check if we have any error function available
            if ctx.has_error_fn() {
                return Self::Error; // Force error mode even if expect(panic) was specified
            } else {
                // Error case - error_type specified but no error function
                panic!(
                    "Field '{}': when struct-level 'error_type' is specified, \
                either provide field-level 'error_fn' or struct-level 'error_fn' as fallback. \
                Example: #[protto(error_type = MyError, error_fn = \"MyError::missing_field\")]",
                    rust.field_name
                );
            }
        }

        // Priority 3: Explicit expect modes (only when no error_type override)
        if rust.expect_mode == ExpectMode::Panic {
            return Self::Panic;
        } else if rust.expect_mode == ExpectMode::Error {
            return Self::Error;
        }

        // Priority 4: Handle remaining fallback patterns
        if Self::custom_functions_need_default_panic(rust) {
            Self::Panic
        } else {
            Self::None
        }
    }

    /// Determine if custom functions should get panic behavior by default
    /// This maintains compatibility with the old system's behavior
    fn custom_functions_need_default_panic(rust: &RustFieldInfo) -> bool {
        // Custom bidirectional functions on complex types got panic behavior in old system
        rust.from_proto_fn.is_some()
            && rust.to_proto_fn.is_some()  // Bidirectional
            && !rust.is_primitive           // Complex type
            && !rust.is_option // Not already optional
    }

    // /// Generate the appropriate error expression based on the error mode and context
    // pub fn generate_error_expression(
    //     &self,
    //     _field_name: &syn::Ident,
    //     proto_field_name: &str,
    //     ctx: &FieldProcessingContext,
    // ) -> proc_macro2::TokenStream {
    //     match self {
    //         ErrorMode::Error => {
    //             if let Some(field_error_fn) = &ctx.field_level_error_fn() {
    //                 // Priority 1: Field-level error_fn
    //                 let error_fn: syn::Path = syn::parse_str(field_error_fn).unwrap();
    //                 quote! { #error_fn(stringify!(#proto_field_name)) }
    //             } else if let Some(struct_error_fn) = ctx.struct_level_error_fn {
    //                 // Priority 2: Struct-level error_fn (when error_type is specified)
    //                 let error_fn: syn::Path = syn::parse_str(struct_error_fn).unwrap();
    //                 quote! { #error_fn(stringify!(#proto_field_name)) }
    //             } else {
    //                 // Priority 3: Default error generation (only when no error_type)
    //                 let default_error_name = &ctx.default_error_ident();
    //                 quote! {
    //                     #default_error_name::MissingField(stringify!(#proto_field_name).to_string())
    //                 }
    //             }
    //         }
    //         ErrorMode::Panic => {
    //             quote! { panic!("Missing required field: {}", stringify!(#proto_field_name)) }
    //         }
    //         ErrorMode::Default(default_fn) => {
    //             if let Some(default_fn_name) = default_fn {
    //                 let default_fn: syn::Path = syn::parse_str(default_fn_name).unwrap();
    //                 quote! { #default_fn() }
    //             } else {
    //                 quote! { Default::default() }
    //             }
    //         }
    //         ErrorMode::None => {
    //             // This shouldn't happen for operations that need error expressions
    //             // But provide a fallback
    //             quote! { Default::default() }
    //         }
    //     }
    // }

    /// Check if this mode requires error handling infrastructure
    #[cfg(test)]
    pub fn requires_error_handling(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// Check if this mode uses default values
    #[cfg(test)]
    pub fn uses_default(&self) -> bool {
        matches!(self, Self::Default(_))
    }

    /// Check if this mode will panic on missing values
    #[cfg(test)]
    pub fn will_panic(&self) -> bool {
        matches!(self, Self::Panic | Self::None)
    }

    /// Get the default function name if specified
    #[cfg(test)]
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
