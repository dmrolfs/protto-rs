# Externalized environment variables configuration - General app development
{ pkgs }:
{
  # Rust development
  RUST_LOG = "info";
  RUST_BACKTRACE = "1";
  CARGO_PROFILE_RELEASE_LTO = "thin";
  CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "1";

  # General application development (not lakehouse-specific)
  # Container development
  CONTAINER_REGISTRY = "localhost:5000";

  # Protocol Buffers
  PROTOC = "${pkgs.protobuf}/bin/protoc";
  PROTOC_INCLUDE = "${pkgs.protobuf}/include";

  # Starship
  STARSHIP_CONFIG = "./nix-config/starship.toml";

  # Editor preferences
  EDITOR = "vim";
  VISUAL = "vim";

  # Pager preferences
  PAGER = "less -R";
  LESS = "-R --use-color -Dd+r -Du+b";

  # Performance
  CARGO_INCREMENTAL = "1";
  RUST_ANALYZER_CARGO_WATCH_COMMAND = "clippy";

  # Development tools
  HYPERFINE_WARMUP_COUNT = "3";

  # API development
  API_PORT = "3000";
  DATABASE_URL = "sqlite://./dev.db";
}
