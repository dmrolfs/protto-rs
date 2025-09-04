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

// DMR: Parse debug configuration once on first call
fn get_debug_mode() -> &'static DebugMode {
    DEBUG_MODE.get_or_init(|| parse_debug_env())
}

// DMR: Parse PROTO_CONVERT_DEBUG environment variable
fn parse_debug_env() -> DebugMode {
    match std::env::var("PROTO_CONVERT_DEBUG") {
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
pub fn should_output_debug(name: impl Display, field_name: impl Display) -> bool {
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
fn matches_debug_pattern(pattern: &str, name: &str) -> bool {
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
        } else if pattern.starts_with('*') {
            // e.g.,  *Request - ends with
            let suffix = &pattern[1..];
            return name.ends_with(suffix);
        } else if pattern.ends_with('*') {
            // e.g., Track* - starts with
            let prefix = &pattern[..pattern.len() - 1];
            return name.starts_with(prefix);
        }
    }

    false
}

/// Print available debug configuration options
#[allow(unused)]
pub fn print_debug_help() {
    eprintln!("ProtoConvert Debug Options:");
    eprintln!("  Environment variable PROTO_CONVERT_DEBUG supports:");
    eprintln!("    all                    # Debug all structs");
    eprintln!("    Request                # Debug exact struct name");
    eprintln!("    Request,Response       # Debug multiple structs (comma-separated)");
    eprintln!("    Track*                 # Debug structs starting with 'Track'");
    eprintln!("    *Request               # Debug structs ending with 'Request'");
    eprintln!("    *Track*                # Debug structs containing 'Track'");
    eprintln!("    Request,Track*,*Response # Combine patterns");
    eprintln!("    0 | false | none       # Disable all debug");
    eprintln!("");
    eprintln!("  Usage during proc macro expansion:");
    eprintln!("    PROTO_CONVERT_DEBUG=Request cargo build");
    eprintln!("    PROTO_CONVERT_DEBUG=Track* cargo test");
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
    pub fn new(function_name: &str, struct_name: impl Display, field_name: impl Display) -> Self {
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
            function_name: function_name.to_string(),
            context,
            depth,
            enabled,
        }
    }

    #[allow(unused)]
    pub fn with_struct_field(
        function_name: &str,
        struct_name: impl Display,
        field: &syn::Field,
    ) -> Self {
        let field_name = field
            .ident
            .as_ref()
            .map(|f| f.to_string())
            .unwrap_or_default();
        Self::new(function_name, struct_name, field_name)
    }

    /// Create a tracker with additional context info
    pub fn with_context(
        function_name: &str,
        struct_name: impl Display,
        field_name: impl Display,
        extra_context: &[(&str, &str)],
    ) -> Self {
        let tracker = Self::new(function_name, struct_name, field_name);

        if tracker.enabled && !extra_context.is_empty() {
            let indent = "  ".repeat(tracker.depth);
            for (key, value) in extra_context {
                eprintln!("{}│  📊 {}: {}", indent, key, value);
            }
        }

        tracker
    }

    /// Log field analysis with structured data
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

    /// Log conversion logic decision tree
    pub fn conversion_logic(&self, conversion_type: &str, decisions: &[(&str, &str)]) -> &Self {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  ⚡ CONVERSION: {}", indent, conversion_type);
            for (condition, result) in decisions {
                eprintln!("{}│    🛤️  {}: {}", indent, condition, result);
            }
        }
        self
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
    pub fn checkpoint(&self, message: &str) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  ✓ {}", indent, message);
        }
    }

    /// Log a checkpoint with data
    pub fn checkpoint_data(&self, message: &str, data: &[(&str, &str)]) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  ✓ {}", indent, message);
            for (key, value) in data {
                eprintln!("{}│    {}: {}", indent, key, value);
            }
        }
    }

    /// Log an error or warning
    pub fn error(&self, message: &str) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  ❌ ERROR: {}", indent, message);
        }
    }

    /// Log an error with context data
    pub fn error_data(&self, message: &str, data: &[(&str, &str)]) {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  ⌘ ERROR: {}", indent, message);
            for (key, value) in data {
                eprintln!("{}│    {}: {}", indent, key, value);
            }
        }
    }

    /// Log a decision point
    pub fn decision(&self, condition: &str, choice: &str) -> &Self{
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  🔀 IF {} THEN {}", indent, condition, choice);
        }
        self
    }

    /// Log conditional expressions for metadata-driven code
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
                let formatted_code = format_rust_code(&expr.to_string());
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
            eprintln!("{}│    🏷️  Proto: {}.{}", indent, proto_message, proto_field);
            eprintln!("{}│    ✅ Result: {:?}", indent, metadata_result);
            eprintln!("{}│    🚩 Strategy: {}", indent, strategy);
        }
        self
    }

    /// Debug error condition (replaces debug_error_condition)
    pub fn error_condition(&self, error_type: &str, details: &str, suggested_fix: Option<&str>) -> &Self {
        if self.enabled {
            let indent = "  ".repeat(self.depth);
            eprintln!("{}│  ❌ ERROR: {}", indent, error_type);
            eprintln!("{}│    📝 Details: {}", indent, details);
            if let Some(fix) = suggested_fix {
                eprintln!("{}│    💡 Fix: {}", indent, fix);
            }
        }
        self
    }

    /// Debug struct-level generation (replaces debug_struct_generation)
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
        "  FROM_PROTO fields: {} lines",
        from_proto_impl.to_string().lines().count()
    );
    eprintln!(
        "  FROM_MY fields: {} lines",
        from_my_impl.to_string().lines().count()
    );

    eprintln!("=== END STRUCT ===\n");
}

/// Debug conversion logic showing the decision tree
pub fn debug_conversion_logic(
    struct_name: impl Display,
    field_name: impl Display,
    conversion_type: &str,
    decision_path: &[(&str, &str)],
    final_expression: &TokenStream,
) {
    let struct_name = struct_name.to_string();
    let field_name = field_name.to_string();

    if !should_output_debug(&struct_name, &field_name) {
        return;
    }

    eprintln!(
        "\n=== ⚡ CONVERSION LOGIC: {}.{} - {} ===",
        struct_name, field_name, conversion_type
    );
    eprintln!("  🛤️  Decision path:");
    for (condition, result) in decision_path {
        eprintln!("    ├─ {}: {}", condition, result);
    }
    eprintln!("  🎯 Final expression:");
    eprintln!("    {}", final_expression);
    eprintln!("=== END CONVERSION LOGIC ===\n");
}

/// Debug struct-level generation
pub fn debug_struct_generation(struct_name: impl Display, phase: &str, info: &[(&str, String)]) {
    let struct_name = struct_name.to_string();
    eprintln!(
        "\n=== 🏢 STRUCT GENERATION: {} - {} ===",
        struct_name, phase
    );
    for (key, value) in info {
        eprintln!("  🔧 {}: {}", key, value);
    }
    eprintln!("=== END STRUCT GENERATION ===\n");
}

/// Debug error conditions
pub fn debug_error_condition(
    struct_name: impl Display,
    field_name: impl Display,
    error_type: &str,
    details: &str,
    suggested_fix: Option<&str>,
) {
    let struct_name = struct_name.to_string();
    let field_name = field_name.to_string();

    if !should_output_debug(&struct_name, &field_name) {
        return;
    }

    eprintln!("\n=== ❌ ERROR: {}.{} ===", struct_name, field_name);
    eprintln!("  🚨 Error type: {}", error_type);
    eprintln!("  📝 Details: {}", details);
    if let Some(fix) = suggested_fix {
        eprintln!("  💡 fix: {}", fix);
    }
    eprintln!("=== END ERROR ===\n");
}

/// Debug the complete generated code with formatted output
pub fn debug_generated_code(
    struct_name: impl Display,
    field_name: impl Display,
    generated_code: &TokenStream,
    context: &str,
    additional_info: &[(&str, &str)],
) {
    let struct_name = struct_name.to_string();
    let field_name = field_name.to_string();

    if !should_output_debug(&struct_name, &field_name) {
        return;
    }

    eprintln!("\n    🏗️  GENERATED CODE: {struct_name}.{field_name} - {context}");

    // Show additional context
    for (key, value) in additional_info {
        eprintln!("    {}: {}", key, value);
    }

    // Format and display the generated code
    let code_str = generated_code.to_string();
    let formatted_code = format_rust_code(&code_str);

    eprintln!("  📝 Generated code:");
    for (i, line) in formatted_code.lines().enumerate() {
        eprintln!("    {:3} | {}", i + 1, line);
    }
    eprintln!("    END GENERATED CODE \n");
}

/// Format generated code for better readability with proper Rust syntax handling
fn format_rust_code(code: &str) -> String {
    let mut result = String::new();
    let mut indent_level = 0;
    let mut chars = code.chars().peekable();
    let mut current_line = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            // Handle opening braces
            '{' => {
                current_line.push(ch);
                result.push_str(&current_line.trim());
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
                if let Some(next_ch) = chars.peek() {
                    if next_ch.is_alphabetic() {
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

/// Alternative simpler formatter for very complex expressions
fn format_rust_code_simple(code: &str) -> String {
    let mut result = String::new();
    let mut indent_level = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for ch in code.chars() {
        if escape_next {
            result.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_string => {
                result.push(ch);
                escape_next = true;
            }
            '"' => {
                result.push(ch);
                in_string = !in_string;
            }
            '{' if !in_string => {
                result.push_str(" {\n");
                indent_level += 1;
                result.push_str(&"    ".repeat(indent_level));
            }
            '}' if !in_string => {
                if !result.ends_with('\n') {
                    result.push('\n');
                }
                indent_level = indent_level.saturating_sub(1);
                result.push_str(&"    ".repeat(indent_level));
                result.push('}');
                result.push('\n');
                if indent_level > 0 {
                    result.push_str(&"    ".repeat(indent_level));
                }
            }
            ';' if !in_string => {
                result.push_str(";\n");
                if indent_level > 0 {
                    result.push_str(&"    ".repeat(indent_level));
                }
            }
            _ => {
                result.push(ch);
            }
        }
    }

    // Clean up extra whitespace and empty lines
    result
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Update the format_generated_code function to use the new formatter
fn format_generated_code(code: &str) -> String {
    // For very long or complex code, use the simple formatter
    if code.len() > 300 || code.matches("::").count() > 5 {
        format_rust_code_simple(code)
    } else {
        format_rust_code(code)
    }
}
