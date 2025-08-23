use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::fs::{self, File};
use std::io::Write;

pub fn generate_proto_metadata<P: AsRef<Path>>(proto_files: &[P]) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = extract_field_metadata(proto_files)?;

    // Debug all CARGO_CFG_FEATURE_* environment variables
    // println!("cargo:warning=proto_convert_build: All CARGO_CFG_FEATURE_* variables:");
    // for (key, value) in std::env::vars() {
    //     if key.starts_with("CARGO_CFG_FEATURE_") {
    //         println!("cargo:warning=  {} = {}", key, value);
    //     }
    // }

    println!("cargo:warning=proto_convert_build: Checking feature flags...");

    // let env_enabled = std::env::var("CARGO_CFG_FEATURE_META_ENV").is_ok();
    // let file_enabled = std::env::var("CARGO_CFG_FEATURE_META_FILE").is_ok();
    let env_enabled = cfg!(feature = "meta-env");
    let file_enabled = cfg!(feature = "meta-file");
    println!("cargo:warning=  meta-env enabled: {}", env_enabled);
    println!("cargo:warning=  meta-file enabled: {}", file_enabled);

    match (env_enabled, file_enabled) {
        (true, true) => {
            return Err("Both 'meta-env' and 'meta-file' are enabled. Please enable only one.".into());
        },
        (true, false) => {
            println!("cargo:warning=Using proto_convert_derive environment variable build-time integration");
            write_metadata_env_vars(metadata)?;
        },
        (false, true) => {
            println!("cargo:warning=Using proto_convert_derive generated file build-time integration (prost-style)");
            write_metadata_file_prost_style(metadata)?;
        },
        (false, false) => {
            println!("cargo:warning=No metadata build-time mechanism enabled. Build-time proto analysis disabled.");
            println!("cargo:warning=  This usually means:");
            println!("cargo:warning=  1. The feature isn't being passed from the example crate");
            println!("cargo:warning=  2. The feature isn't properly declared in workspace");
            println!("cargo:warning=  3. Build script is running in wrong crate context");
        },
    }

    // tell cargo to rerun if proto files change
    for proto_file in proto_files {
        println!("cargo:rerun-if-changed={}", proto_file.as_ref().display());
    }

    Ok(())
}

/// Extract field optionality from .proto files using simple regex parsing
fn extract_field_metadata<P: AsRef<Path>>(proto_files: &[P]) -> Result<ProtoMetadata, Box<dyn std::error::Error>> {
    let mut metadata = ProtoMetadata::default();

    for proto_file in proto_files {
        let content = fs::read_to_string(proto_file)?;
        parse_proto_content(&content, &mut metadata)?;
    }

    Ok(metadata)
}

#[derive(Debug, Default)]
struct ProtoMetadata {
    // message_name -> field_name -> FieldInfo
    messages: HashMap<String, HashMap<String, FieldInfo>>,
}

impl ProtoMetadata {
    fn add_field(&mut self, message: String, field: String, info: FieldInfo) {
        self.messages
            .entry(message)
            .or_insert_with(HashMap::new)
            .insert(field, info);
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct FieldInfo {
    optional: bool,
    repeated: bool,
    field_type: String,
}

fn parse_proto_content(content: &str, metadata: &mut ProtoMetadata) -> Result<(), Box<dyn std::error::Error>> {
    use regex::Regex;

    let message_regex = Regex::new(r"message\s+(\w+)\s*\{")?;
    let enum_regex = Regex::new(r"enum\s+(\w+)\s*\{")?;
    let field_regex = Regex::new(r"^\s*(optional|required|repeated)?\s*(\w+)\s+(\w+)\s*=\s*\d+;")?;

    let mut current_message: Option<String> = None;
    let mut known_enums = HashSet::new();
    let mut brace_depth = 0;

    for line in content.lines() {
        let line = line.trim();

        // skip comments and empty lines
        if line.starts_with("//") || line.starts_with("/*") || line.is_empty() {
            continue;
        }

        // track enum definitions
        if let Some(caps) = enum_regex.captures(line) {
            known_enums.insert(caps[1].to_string());
            continue;
        }

        // track message boundaries
        if let Some(caps) = message_regex.captures(line) {
            current_message = Some(caps[1].to_string());
            brace_depth = 1;
            continue;
        }

        // track braces to handle nested messages
        if current_message.is_some() {
            // track braces to handle nested messages
            brace_depth += line.chars().filter(|&c| c == '{').count();
            brace_depth -= line.chars().filter(|&c| c == '}').count();

            if brace_depth == 0 {
                current_message = None;
                continue;
            }
        }

        // parse field definitions
        if let (Some(ref message_name), Some(caps)) = (&current_message, field_regex.captures(line)) {
            let modifier = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let field_type = caps[2].to_string();
            let field_name = caps[3].to_string();

            let field_info = FieldInfo {
                optional: modifier == "optional" ||
                    (modifier.is_empty() && is_likely_optional(&field_type, &known_enums)),
                repeated: modifier == "repeated",
                field_type,
            };

            metadata.add_field(message_name.clone(), field_name, field_info);
        }
    }

    Ok(())
}

/// Proto3 heuristic: message types are implicitly optional, primitives are not
fn is_likely_optional(field_type: &str, known_enums: &HashSet<String>) -> bool {
    // check if it's a primitive type first
    let is_primitive = matches!(
        field_type,
        "int32" | "int64" | "uint32" | "uint64" |
        "sint32" | "sint64" | "fixed32" | "fixed64" |
        "sfixed32" | "sfixed64" | "float" | "double" |
        "bool" | "string" | "bytes"
    );

    let is_enum = known_enums.contains(field_type);

    !is_primitive && !is_enum
}

/// Generate the metadata that the macro can include.
/// Write metadata using environment variables approach.
///
/// This generates environment variables that can be read by the proc macro
/// at compile time. Each proto field gets an environment variable:
/// `PROTO_FIELD_{MESSAGE}_{FIELD}={optional|required|repeated}`
///
/// ## Migration to Prost-Style
///
/// To migrate to prost-style file inclusion, replace this function with
/// `write_metadata_file_prost_style()` and update the proc macro to use
/// `include!(concat!(env!("OUT_DIR"), "/proto_field_metadata.rs"))`.
///
/// ### Environment Variable Limits:
/// - Windows: ~32KB per environment variable
/// - Linux/macOS: Much higher (typically 128KB+)
/// - If you hit these limits, migrate to file inclusion approach
fn write_metadata_env_vars(metadata: ProtoMetadata) -> Result<(), Box<dyn std::error::Error>> {
    let mut var_count = 0;
    let mut total_size = 0;

    for (message_name, fields) in &metadata.messages {
        for (field_name, field_info) in fields {
            let env_key = format!(
                "PROTO_FIELD_{}_{}",
                message_name.to_uppercase(), field_name.to_uppercase()
            );

            let env_value = if field_info.repeated {
                "repeated"
            } else if field_info.optional {
                "optional"
            } else {
                "required"
            };

            // set environment variable for proc macro to read
            println!("cargo:rustc-env={}={}", env_key, env_value);

            var_count += 1;
            total_size += env_key.len() + env_value.len() + 2; // +2 for = and newline
        }
    }

    // warn if approaching environment vairable limitis
    if 100 < var_count {
        println!(
            "cargo:warning=proto-convert: {var_count} environment variables generated. \
            Consider migrating to prost-style file inclusion for better performance."
        );
    }

    if 25_000 < total_size {
        println!(
            "cargo:warning=proto-convert: {}KB of metadata generated. \
            Approaching environment variable size limits. \
            Consider migrating to prost-style file inclusion.",
            total_size / 1024
        );
    }

    println!("cargo:rustc-env=PROTO_METADATA_COUNT={var_count}");

    Ok(())
}

/// Alternative implementation for prost-style file inclusion (for migration).
///
/// This generates a Rust source file with static data structures instead of
/// environment variables. Use this when environment variables become limiting.
///
/// ## Usage:
/// 1. Replace `write_metadata_env_vars()` call with this function
/// 2. Update proc macro to use `FileInclusionMetadataProvider`
/// 3. Add consumer boilerplate to include generated file
fn write_metadata_file_prost_style(metadata: ProtoMetadata) -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=DEBUG: write_metadata_file_prost_style called");
    println!("cargo:warning=DEBUG: Generating metadata file with {} messages", metadata.messages.len());

    // Debug the metadata content
    for (msg, fields) in &metadata.messages {
        println!("cargo:warning=DEBUG: Message '{}' has {} fields", msg, fields.len());
        for (field, info) in fields {
            println!("cargo:warning=DEBUG:   {}.{} -> optional={}", msg, field, info.optional);
        }
    }

    let out_dir = std::env::var("OUT_DIR").map_err(|e| {
        format!("Failed to get OUT_DIR environment variable: {}", e)
    })?;
    println!("cargo:warning=DEBUG: OUT_DIR = {}", out_dir);

    let dest_path = Path::new(&out_dir).join("proto_field_metadata.rs");
    println!("cargo:warning=DEBUG: Will write to: {}", dest_path.display());

    let mut f = File::create(&dest_path).map_err(|e| {
        format!("Failed to create metadata file at {}: {}", dest_path.display(), e)
    })?;

    writeln!(f, "// Generated proto field metadata by proto_convert_build - do not edit")?;
    // writeln!(f, "// Generated at: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))?;
    writeln!(f, "")?;

    writeln!(f, "use std::collections::HashMap;")?;
    writeln!(f, "")?;

    writeln!(f, "/// Get field optionality for a proto message field")?;
    writeln!(f, "pub fn get_field_optionality(message: &str, field: &str) -> Option<bool> {{")?;
    writeln!(f, "    match (message, field) {{")?;

    let mut field_count = 0;
    for (message_name, fields) in &metadata.messages {
        for (field_name, field_info) in fields {
            writeln!(
                f,
                "        (\"{}\", \"{}\") => Some({}),",
                message_name, field_name, field_info.optional
            )?;
            field_count += 1;
        }
    }

    writeln!(f, "        _ => None,")?;
    writeln!(f, "    }}")?;
    writeln!(f, "}}")?;
    writeln!(f, "")?;

    // DMR: Optional debug function
    writeln!(f, "/// Get all metadata (for debugging)")?;
    writeln!(f, "#[allow(dead_code)]")?;
    writeln!(f, "pub fn get_all_metadata() -> &'static [(&'static str, &'static [(&'static str, bool)])] {{")?;
    writeln!(f, "    &[")?;
    for (message_name, fields) in &metadata.messages {
        writeln!(f, "        (\"{}\", &[", message_name)?;
        for (field_name, field_info) in fields {
            writeln!(f, "            (\"{}\", {}),", field_name, field_info.optional)?;
        }
        writeln!(f, "        ]),")?;
    }
    writeln!(f, "    ]")?;
    writeln!(f, "}}")?;

    f.flush()?; // Ensure file is written

    println!("cargo:warning=proto_convert_build: Successfully generated metadata file at {}", dest_path.display());
    println!("cargo:warning=proto_convert_build: Generated {} field mappings", field_count);
    Ok(())
}