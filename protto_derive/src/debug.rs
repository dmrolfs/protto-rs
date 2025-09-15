//! # Debug Module
//!
//! Advanced debugging and introspection capabilities for the `protto_derive` procedural macro.
//!
//! ## Overview
//!
//! The debug module provides comprehensive debugging tools to help understand and troubleshoot
//! the code generation process during macro expansion. It offers structured logging, call stack
//! tracking, and detailed insights into type analysis and conversion strategy selection.
//!
//! ## Features
//!
//! - **Environment-Controlled Debugging**: Enable/disable debugging via `PROTTO_DEBUG` environment variable
//! - **Selective Debugging**: Target specific structs using pattern matching
//! - **Call Stack Tracking**: Automatic function entry/exit logging with proper indentation
//! - **Code Generation Visualization**: Pretty-printed generated Rust code with syntax highlighting
//! - **Type Analysis Logging**: Detailed insights into type resolution and conversion decisions
//! - **Performance Tracking**: Minimal overhead when debugging is disabled
//!
//! ## Quick Start
//!
//! **⚠️ Critical**: Always run `cargo clean` before enabling debug output. Since debug output
//! occurs during macro expansion at compile time, if your code is already compiled, no debug
//! information will be shown even with the environment variable set.
//!
//! Enable debugging for all structs during compilation:
//! ```bash
//! cargo clean  # Essential: clean first to force recompilation
//! PROTTO_DEBUG=all cargo build
//! ```
//!
//! Debug specific structs:
//!
//! ```bash
//! PROTTO_DEBUG=Request,Response cargo test
//! ```
//!
//! Debug with pattern matching:
//!
//! ```bash
//! PROTTO_DEBUG="Track*,*Request" cargo build
//! ```
//!
//! ## Configuration
//!
//! The `PROTTO_DEBUG` environment variable supports various patterns:
//!
//! | Pattern | Description | Example |
//! |---------|-------------|---------|
//! | `all` | Debug all structs | `PROTTO_DEBUG=all` |
//! | `StructName` | Exact struct name | `PROTTO_DEBUG=Request` |
//! | `Pattern*` | Prefix match | `PROTTO_DEBUG=Track*` |
//! | `*Pattern` | Suffix match | `PROTTO_DEBUG=*Request` |
//! | `*Pattern*` | Contains match | `PROTTO_DEBUG=*User*` |
//! | `A,B,C` | Multiple patterns | `PROTTO_DEBUG=Request,Track*,*Response` |
//! | `0\|false\|none` | Disable debugging | `PROTTO_DEBUG=false` |
//!
//! ## Debug Output Structure
//!
//! The debug output uses a hierarchical tree structure with Unicode box-drawing characters:
//!
//! ```text
//! ┌─ ENTER: generate_field_conversions [Request.header]
//! │  📊 strategy: TransparentOptionalWithExpect
//! │  📊 debug_info: proto optional -> rust required + expect
//! │  ✓ field_analysis
//! │    category: TransparentConversion
//! │    debug_info: proto optional -> rust required + expect
//! │  🔀 IF TransparentOptionalWithExpected THEN expect with panic message
//! │  🛠️ Generated code:
//!     1 | header: proto::Header::from(
//!     2 |     proto_struct.header
//!     3 |         .expect(&format!("Proto field header is required"))
//!     4 | )
//! └─ EXIT:  generate_field_conversions [Request.header]
//! ```
//!
//! ## Programming Interface
//!
//! ### CallStackDebug
//!
//! The main debugging utility that automatically tracks function calls:
//!
//! ```rust,ignore
//! use crate::debug::CallStackDebug;
//!
//! fn my_function(ctx: &Context) -> TokenStream {
//!     let _trace = CallStackDebug::new(
//!         "my_function",
//!         ctx.struct_name,
//!         ctx.field_name
//!     );
//!
//!     _trace.checkpoint("Starting analysis");
//!
//!     // Your logic here
//!
//!     _trace.decision("has_custom_conversion", "use custom function");
//!
//!     let result = quote! { /* generated code */ };
//!
//!     _trace.generated_code(&result, ctx.struct_name, ctx.field_name, "conversion", &[
//!         ("strategy", "CustomFunction"),
//!         ("direction", "proto_to_rust"),
//!     ]);
//!
//!     result
//! } // Automatically logs EXIT when _trace is dropped
//! ```
//!
//! ### Debugging Methods
//!
//! #### Basic Logging
//! - `checkpoint(message)` - Log a progress checkpoint
//! - `checkpoint_data(message, data)` - Checkpoint with key-value data
//! - `decision(condition, choice)` - Log decision points in code generation
//! - `error(message)` - Log errors or warnings
//!
//! #### Specialized Logging
//! - `generated_code()` - Pretty-print generated Rust code
//! - `type_analysis()` - Log type resolution analysis
//! - `type_mismatch()` - Debug type compatibility issues
//! - `metadata_lookup()` - Track proto metadata queries
//!
//! ### Constructor Variants
//!
//! ```rust,ignore
//! // Basic constructor
//! let trace = CallStackDebug::new("function_name", struct_name, field_name);
//!
//! // With additional context
//! let trace = CallStackDebug::with_context("function_name", struct_name, field_name, &[
//!     ("strategy", "DirectConversion"),
//!     ("optionality", "Required"),
//! ]);
//!
//! // With syn::Field
//! let trace = CallStackDebug::with_struct_field("function_name", struct_name, &field);
//! ```
//!
//! ## Code Generation Debugging
//!
//! The debug system provides detailed insights into the code generation process:
//!
//! ### Field Analysis
//! ```text
//! │  🎯 TYPE ANALYSIS
//! │    🦀 Rust: Option<String>
//! │    📦 Proto field: name
//! │    📦 Proto type: Option<String>
//! │    📦 Proto mapping: Optional
//! ```
//!
//! ### Decision Trees
//! ```text
//! │  🔀 IF proto_field.is_optional() THEN UnwrapOptionalWithExpect
//! │  🔀 IF rust_field.is_option THEN MapOption
//! │  🔀 IF has_custom_conversion THEN DeriveBidirectional
//! ```
//!
//! ### Generated Code
//! The debug output includes formatted Rust code with line numbers:
//! ```text
//! │  🛠️ Generated code: Request.header - proto_to_rust
//! │    📌 strategy: TransparentOptionalWithExpect
//! │    📌 direction: proto_to_rust
//!     1 | header: proto::Header::from(
//!     2 |     proto_struct.header
//!     3 |         .expect(&format!(
//!     4 |             "Proto field header is required for transparent conversion"
//!     5 |         ))
//!     6 | )
//! ```
//!
//! ## Performance Considerations
//!
//! - **Zero Runtime Cost**: All debugging is compile-time only
//! - **Lazy Evaluation**: Debug checks are performed only once per compilation
//! - **Minimal Overhead**: When debugging is disabled, most operations are no-ops
//! - **Structured Output**: Efficient string formatting with minimal allocations
//!
//! ## Common Use Cases
//!
//! ### Debugging Conversion Issues
//! ```bash
//! # Debug a specific problematic struct
//! PROTTO_DEBUG=MyStruct cargo build 2>&1 | less
//! ```
//!
//! ### Understanding Type Resolution
//! ```bash
//! # Debug all structures with "User" in the name
//! PROTTO_DEBUG="*User*" cargo test
//! ```
//!
//! ### Investigating Generated Code
//! ```bash
//! # Debug multiple related structs
//! PROTTO_DEBUG="Request,Response,*Header" cargo build --verbose
//! ```
//!
//! ### Performance Analysis
//! ```bash
//! # Debug all structs to see the full conversion process
//! PROTTO_DEBUG=all cargo build --release 2>&1 | grep "GENERATED CODE"
//! ```
//!
//! ## Output Interpretation
//!
//! ### Symbols and Their Meanings
//! - `┌─` / `└─` - Function entry/exit
//! - `│` - Call stack depth indicator
//! - `📊` - Context information
//! - `✓` - Successful checkpoint
//! - `🔀` - Decision point
//! - `🎯` - Type analysis
//! - `🛠️` - Generated code
//! - `⚠️` - Warning or error
//! - `💡` - Suggestion or fix
//!
//! ### Reading the Call Stack
//! The indentation level indicates function call depth:
//! ```text
//! ┌─ ENTER: analyze_struct [MyStruct.]
//! │  ┌─ ENTER: analyze_field [MyStruct.field1]
//! │  │  ✓ Field analysis complete
//! │  └─ EXIT:  analyze_field [MyStruct.field1]
//! │  ┌─ ENTER: analyze_field [MyStruct.field2]
//! │  │  ✓ Field analysis complete
//! │  └─ EXIT:  analyze_field [MyStruct.field2]
//! └─ EXIT:  analyze_struct [MyStruct.]
//! ```
//!
//! ## Environment Variables
//!
//! - `PROTTO_DEBUG` - Main debug control (see Configuration section)
//! - Standard Rust logging variables (`RUST_LOG`) work alongside this system
//!
//! ## Integration with IDEs
//!
//! The debug output is designed to be readable in:
//! - Terminal output with color support
//! - IDE build output panels
//! - Log files (maintains structure without colors)
//! - Error parsing tools (clear file/line information when relevant)
//!
//! ## Troubleshooting
//!
//! ### Debug Not Working
//! 1. Ensure the environment variable is set correctly
//! 2. Verify the pattern matches your struct names
//! 3. Check that macro expansion is actually occurring
//!
//! ### Too Much Output
//! 1. Use more specific patterns instead of `all`
//! 2. Pipe output through `grep` to filter specific sections
//! 3. Use `less` or similar pagers for navigation
//!
//! ### Performance Impact
//! Debug mode only affects compilation time, not runtime performance.
//! However, very verbose debugging can slow down compilation significantly.
//!
//! ## Examples
//!
//! See the integration tests and examples directory for complete usage examples
//! demonstrating various debugging scenarios and patterns.

use proc_macro2::TokenStream;
use std::fmt::Display;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone)]
enum DebugMode {
    Disabled,
    All,
    Patterns(Vec<String>),
}

static DEBUG_MODE: OnceLock<DebugMode> = OnceLock::new();

// Parse debug configuration once on first call
fn get_debug_mode() -> &'static DebugMode {
    DEBUG_MODE.get_or_init(parse_debug_env)
}

// Parse PROTTO_DEBUG environment variable
fn parse_debug_env() -> DebugMode {
    match std::env::var("PROTTO_DEBUG") {
        Ok(env_debug) => match env_debug.as_str() {
            "1" | "true" | "all" => DebugMode::All,
            "0" | "false" | "none" | "" => DebugMode::Disabled,
            _ => {
                let patterns = env_debug
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                DebugMode::Patterns(patterns)
            }
        },
        Err(_) => DebugMode::Disabled,
    }
}

/// Check if debug output should be enabled for a specific struct/field combination
pub fn should_output_debug(name: impl Display, _field_name: impl Display) -> bool {
    let name = name.to_string();

    match get_debug_mode() {
        DebugMode::Disabled => false,
        DebugMode::All => true,
        DebugMode::Patterns(patterns) => patterns
            .iter()
            .any(|pattern| matches_debug_pattern(pattern, &name)),
    }
}

/// Check if a struct name matches a debug pattern
/// Supports:
/// - Exact match: "Request"
/// - Prefix glob: "Track*"
/// - Suffix glob: "*Request"
/// - Contains glob: "*Track*"
/// - Multiple patterns: "Request,Track*,*Response"
fn matches_debug_pattern(pattern: impl AsRef<str>, name: impl AsRef<str>) -> bool {
    let pattern = pattern.as_ref();
    let name = name.as_ref();

    if pattern == "all" {
        return true;
    }

    // Exact match
    if pattern == name {
        return true;
    }

    // Glob patterns
    if pattern.contains('*') {
        if pattern.starts_with('*') && pattern.ends_with('*') {
            // e.g.,  *Track* - contains
            let middle = &pattern[1..pattern.len() - 1];
            return name.contains(middle);
        } else if let Some(suffix) = pattern.strip_prefix('*') {
            // e.g.,  *Request - ends with
            return name.ends_with(suffix);
        } else if let Some(prefix) = pattern.strip_suffix('*') {
            // e.g., Track* - starts with
            return name.starts_with(prefix);
        }
    }

    false
}

/// Print available debug configuration options
#[allow(unused)]
pub fn print_debug_help() {
    eprintln!("Protto Debug Options:");
    eprintln!("  Environment variable PROTTO_DEBUG supports:");
    eprintln!("    all                    # Debug all structs");
    eprintln!("    Request                # Debug exact struct name");
    eprintln!("    Request,Response       # Debug multiple structs (comma-separated)");
    eprintln!("    Track*                 # Debug structs starting with 'Track'");
    eprintln!("    *Request               # Debug structs ending with 'Request'");
    eprintln!("    *Track*                # Debug structs containing 'Track'");
    eprintln!("    Request,Track*,*Response # Combine patterns");
    eprintln!("    0 | false | none       # Disable all debug");
    eprintln!();
    eprintln!("  Usage during proc macro expansion:");
    eprintln!("    PROTTO_DEBUG=Request cargo build");
    eprintln!("    PROTTO_DEBUG=Track* cargo test");
}

// Global call depth counter for indentation
static CALL_DEPTH: AtomicUsize = AtomicUsize::new(0);

/// Debug tracker that logs function entry on creation and exit on drop
pub struct CallStackDebug {
    function_name: String,
    context: String,
    depth: usize,
    enabled: bool,
}

impl CallStackDebug {
    /// Create a new call stack tracker
    pub fn new(
        code_module: &str,
        function_name: &str,
        struct_name: impl Display,
        field_name: impl Display
    ) -> Self {
        let function_name = format!("{code_module}::{function_name}");
        let struct_name = struct_name.to_string();
        let field_name = field_name.to_string();
        let context = format!("{}.{}", struct_name, field_name);

        // Check if debugging is enabled for this struct/field
        let enabled = should_output_debug(&struct_name, &field_name);

        let depth = CALL_DEPTH.fetch_add(1, Ordering::SeqCst);

        if enabled {
            let indent = "  ".repeat(depth);
            eprintln!("{}┌─ ENTER: {} [{}]", indent, function_name, context);
        }

        Self {
            function_name,
            context,
            depth,
            enabled,
        }
    }

    #[allow(unused)]
    pub fn with_struct_field(
        code_module: &str,
        function_name: &str,
        struct_name: impl Display,
        field: &syn::Field,
    ) -> Self {
        let field_name = field
            .ident
            .as_ref()
            .map(|f| f.to_string())
            .unwrap_or_default();
        Self::new(code_module, function_name, struct_name, field_name)
    }

    /// Create a tracker with additional context info
    pub fn with_context(
        code_module: &str,
        function_name: &str,
        struct_name: impl Display,
        field_name: impl Display,
        extra_context: &[(&str, &str)],
    ) -> Self {
        let tracker = Self::new(code_module, function_name, struct_name, field_name);

        if tracker.enabled && !extra_context.is_empty() {
            let indent = "  ".repeat(tracker.depth);
            for (key, value) in extra_context {
                eprintln!("{}│  📊 {}: {}", indent, key, value);
            }
        }

        tracker
    }

    /// Log field analysis with structured data
    #[allow(unused)]
    pub fn field_analysis(&self, phase: &str, data: &[(&str, &str)]) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  🔍 {}", indent, phase);
            for (key, value) in data {
                eprintln!("{}│    📋 {}: {}", indent, key, value);
            }
        }
    }

    /// Log type resolution analysis
    #[allow(unused)]
    pub fn type_analysis(&self, rust_type: &str, proto_info: &[(&str, &str)]) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  🎯 TYPE ANALYSIS", indent);
            eprintln!("{}│    🦀 Rust: {}", indent, rust_type);
            for (key, value) in proto_info {
                eprintln!("{}│    📦 {}: {}", indent, key, value);
            }
        }
    }

    /// Log generated code with context
    pub fn generated_code(
        &self,
        code: &TokenStream,
        struct_name: impl Display,
        field_name: impl Display,
        context: &str,
        info: &[(&str, &str)],
    ) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);

            eprintln!(
                "\n{indent}=== 🛠️ GENERATED CODE: {struct_name}.{field_name} - {context} ==="
            );
            // eprintln!("{}│  📝 GENERATED: {}", indent, context);

            for (key, value) in info {
                eprintln!("{}│    📌 {}: {}", indent, key, value);
            }

            let code_str = code.to_string();
            let formatted_code = format_rust_code(&code_str);
            eprintln!("{}│    📝 Generated code:", indent);
            for (i, line) in formatted_code.lines().enumerate() {
                eprintln!("    {:3} | {}", i + 1, line);
            }
            eprintln!("{indent}=== END GENERATED CODE ===\n");
        }
    }

    /// Log a checkpoint within the function
    pub fn checkpoint(&self, message: impl AsRef<str>) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  ✓ {}", indent, message.as_ref());
        }
    }

    /// Log a checkpoint with data
    pub fn checkpoint_data(&self, message: impl AsRef<str>, data: &[(&str, &str)]) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  ✓ {}", indent, message.as_ref());
            for (key, value) in data {
                eprintln!("{}│    {}: {}", indent, key, value);
            }
        }
    }

    /// Log an error or warning
    pub fn error(&self, message: impl AsRef<str>) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  ❌ ERROR: {}", indent, message.as_ref());
        }
    }

    /// Log an error with context data
    #[allow(unused)]
    pub fn error_data(&self, message: impl AsRef<str>, data: &[(&str, &str)]) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  ⌘ ERROR: {}", indent, message.as_ref());
            for (key, value) in data {
                eprintln!("{}│    {}: {}", indent, key, value);
            }
        }
    }

    /// Log a decision point
    pub fn decision(&self, condition: &str, choice: &str) -> &Self {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  🔀 IF {} THEN {}", indent, condition, choice);
        }
        self
    }

    /// Log conditional expressions for metadata-driven code
    #[allow(unused)]
    pub fn conditional_exprs(
        &self,
        label: &str,
        optionality: impl Display,
        exprs: &[(&str, &TokenStream)],
    ) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  🔀 CONDITIONAL: {} ({})", indent, label, optionality);
            for (name, expr) in exprs {
                let formatted_code = format_rust_code(expr.to_string());
                eprintln!("{}│    📝 Generated code:", indent);
                eprintln!("{}│       {} -> {{", indent, name);
                for (i, line) in formatted_code.lines().enumerate() {
                    eprintln!("    {:3} | {}", i + 1, line);
                }
                eprintln!("{}│       }}", indent);
            }
        }
    }

    /// Debug type mismatch analysis (replaces debug_type_mismatch_analysis)
    #[allow(unused)]
    pub fn type_mismatch(
        &self,
        expected_rust_type: &str,
        actual_proto_type: &str,
        conversion_attempt: &str,
        suggested_fixes: &[&str],
    ) -> &Self {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  🎯 TYPE MISMATCH", indent);
            eprintln!("{}│    🦀 Expected: {}", indent, expected_rust_type);
            eprintln!("{}│    📦 Actual: {}", indent, actual_proto_type);
            eprintln!("{}│    🔄 Attempted: {}", indent, conversion_attempt);
            if !suggested_fixes.is_empty() {
                eprintln!("{}│    💡 Fixes: {}", indent, suggested_fixes.join(", "));
            }
        }
        self
    }

    /// Debug type resolution (replaces debug_type_resolution)
    #[allow(unused)]
    pub fn type_resolution(
        &self,
        rust_type: &str,
        proto_field_name: &str,
        proto_type_info: &str,
        metadata_result: Option<bool>,
    ) -> &Self {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  🎯 TYPE RESOLUTION", indent);
            eprintln!("{}│    🦀 Rust: {}", indent, rust_type);
            eprintln!("{}│    📦 Proto field: {}", indent, proto_field_name);
            eprintln!("{}│    🔧 Type info: {}", indent, proto_type_info);
            eprintln!("{}│    📋 Metadata: {:?}", indent, metadata_result);
        }
        self
    }

    /// Debug metadata lookup (replaces debug_metadata_lookup)
    #[allow(unused)]
    pub fn metadata_lookup(
        &self,
        proto_message: &str,
        proto_field: &str,
        metadata_result: Option<bool>,
        strategy: &str,
    ) -> &Self {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  📋 METADATA LOOKUP", indent);
            eprintln!(
                "{}│    🏷️  Proto: {}.{}",
                indent, proto_message, proto_field
            );
            eprintln!("{}│    ✅ Result: {:?}", indent, metadata_result);
            eprintln!("{}│    🚩 Strategy: {}", indent, strategy);
        }
        self
    }

    /// Debug error condition (replaces debug_error_condition)
    #[allow(unused)]
    pub fn error_condition(
        &self,
        error_type: &str,
        details: impl AsRef<str>,
        suggested_fix: Option<&str>,
    ) -> &Self {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  ❌ ERROR: {}", indent, error_type);
            eprintln!("{}│    📝 Details: {}", indent, details.as_ref());
            if let Some(fix) = suggested_fix {
                eprintln!("{}│    💡 Fix: {}", indent, fix);
            }
        }
        self
    }

    /// Debug struct-level generation (replaces debug_struct_generation)
    #[allow(unused)]
    pub fn struct_generation(&self, phase: &str, info: &[(&str, &str)]) -> &Self {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  🏢 STRUCT: {}", indent, phase);
            for (key, value) in info {
                eprintln!("{}│    🔧 {}: {}", indent, key, value);
            }
        }
        self
    }
}

impl Drop for CallStackDebug {
    fn drop(&mut self) {
        // Decrement depth before logging exit
        CALL_DEPTH.fetch_sub(1, Ordering::SeqCst);

        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!(
                "{}└─ EXIT:  {} [{}]",
                indent, self.function_name, self.context
            );
        }
    }
}

pub fn debug_struct_conversion_generation(
    struct_name: impl Display,
    phase: &str,
    from_proto_impl: &TokenStream,
    from_my_impl: &TokenStream,
    final_impl: &TokenStream,
    additional_info: &[(&str, String)],
) {
    let name = struct_name.to_string();
    if !should_output_debug(&name, "") {
        return;
    }

    eprintln!("\n=== 🏢 STRUCT: {} - {} ===", name, phase);

    for (key, value) in additional_info {
        eprintln!("  📊 {}: {}", key, value);
    }

    // Only show structure, not full implementation
    eprintln!(
        "  FROM_PROTO: {}",
        format_rust_code(from_proto_impl.to_string())
    );
    eprintln!("  FROM_MY: {}", format_rust_code(from_my_impl.to_string()));
    eprintln!(
        "  FINAL_IMPL:\n{}",
        format_rust_code(final_impl.to_string())
    );

    eprintln!("=== END STRUCT ===\n");
}

/// Format generated code for better readability with proper Rust syntax handling
fn format_rust_code(code: impl AsRef<str>) -> String {
    let code = code.as_ref();
    let mut result = String::new();
    let mut indent_level = 0;
    let mut chars = code.chars().peekable();
    let mut current_line = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            // Handle opening braces
            '{' => {
                current_line.push(ch);
                result.push_str(current_line.trim());
                result.push('\n');
                indent_level += 1;
                current_line.clear();
            }

            // Handle closing braces
            '}' => {
                if !current_line.trim().is_empty() {
                    result.push_str(&format!(
                        "{}{}\n",
                        "    ".repeat(indent_level),
                        current_line.trim()
                    ));
                    current_line.clear();
                }
                indent_level = indent_level.saturating_sub(1);
                result.push_str(&format!("{}}}", "    ".repeat(indent_level)));

                // Check if there's more content after the brace
                if chars.peek().is_some() {
                    result.push('\n');
                }
            }

            // Handle semicolons - end of statement
            ';' => {
                current_line.push(ch);
                result.push_str(&format!(
                    "{}{}\n",
                    "    ".repeat(indent_level),
                    current_line.trim()
                ));
                current_line.clear();
            }

            // Handle commas in function calls and method chains
            ',' => {
                current_line.push(ch);
                // Only break on comma if we're not in a method chain context
                if !is_in_method_chain(&current_line) {
                    result.push_str(&format!(
                        "{}{}\n",
                        "    ".repeat(indent_level),
                        current_line.trim()
                    ));
                    current_line.clear();
                } else {
                    current_line.push(' ');
                }
            }

            // Handle method chaining with dots
            '.' => {
                current_line.push(ch);
                // Look ahead to see if this starts a new method call
                if let Some(next_ch) = chars.peek()
                    && next_ch.is_alphabetic()
                {
                    // This is a method call, check if we should break the line
                    if current_line.trim().len() > 60 {
                        // Break long method chains
                        result.push_str(&format!(
                            "{}{}\n",
                            "    ".repeat(indent_level),
                            current_line.trim()
                        ));
                        current_line.clear();
                        current_line.push_str(&format!("{}.", "    ".repeat(indent_level + 1)));
                    }
                }
            }

            // Handle question marks (for Result types)
            '?' => {
                current_line.push(ch);
                // Look ahead to see what follows
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '.' {
                        // Continue the method chain
                        current_line.push(' ');
                    } else if next_ch.is_whitespace() || next_ch == ';' {
                        // End of expression
                        current_line.push(' ');
                    }
                }
            }

            // Handle whitespace - collapse multiple spaces but preserve structure
            ch if ch.is_whitespace() => {
                if !current_line.ends_with(' ') && !current_line.is_empty() {
                    current_line.push(' ');
                }
            }

            // Regular characters
            _ => {
                current_line.push(ch);
            }
        }
    }

    // Add any remaining content
    if !current_line.trim().is_empty() {
        result.push_str(&format!(
            "{}{}",
            "    ".repeat(indent_level),
            current_line.trim()
        ));
    }

    // Clean up the result
    cleanup_formatting(&result)
}

/// Check if we're in a method chain context where we shouldn't break on commas
fn is_in_method_chain(line: &str) -> bool {
    let trimmed = line.trim();
    // Don't break on commas inside function parameters or generic type parameters
    let open_parens = trimmed.matches('(').count();
    let close_parens = trimmed.matches(')').count();
    let open_angles = trimmed.matches('<').count();
    let close_angles = trimmed.matches('>').count();

    // We're inside parentheses or angle brackets
    open_parens > close_parens || open_angles > close_angles
}

/// Clean up the formatted output
fn cleanup_formatting(code: &str) -> String {
    code.lines()
        .map(|line| line.trim_end()) // Remove trailing whitespace
        .filter(|line| !line.trim().is_empty() || code.lines().count() < 5) // Remove empty lines unless it's very short code
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\n\n\n", "\n\n") // Collapse multiple empty lines
        .trim()
        .to_string()
}
