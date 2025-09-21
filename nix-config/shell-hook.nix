# Externalized shell hook configuration
{ pkgs, rustToolchain }:
let
  # Import aliases
  aliases = import ./shell-aliases.nix;
  # Convert alias set to shell alias commands
  aliasCommands = pkgs.lib.concatStringsSep "\n"
    (pkgs.lib.mapAttrsToList (name: value: "alias ${name}=\"${value}\"") aliases);
in
''
  # Initialize starship
  eval "$(starship init bash)"

  # Set up all aliases from imported configuration
  ${aliasCommands}

  # Set up git hooks if pre-commit is available
  if command -v pre-commit >/dev/null 2>&1 && [ -f .pre-commit-config.yaml ]; then
    pre-commit install --install-hooks 2>/dev/null || true
  fi

  # Set RUST_SRC_PATH for IDE integration
  export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"

  # DMR: Enhanced welcome message for general app development
  echo "🦀 Comprehensive Rust Development Environment"
  echo "📦 Rust toolchain: $(rustc --version)"
  echo "🔧 protoc version: $(protoc --version)"
  echo "🐳 Container tools: podman, podman-compose, drill"
  echo "✨ Modern CLI tools: rg, fd, lnav, bat, eza, procs, dust, htop, tokei, etc."
  echo ""
  echo "🏗️  Available commands:"
  echo "  just --list          # Show available tasks"
  echo "  cargo build          # Build the project"
  echo "  cargo test           # Run tests"
  echo "  cargo nextest run    # Run tests with nextest"
  echo "  cargo tarpaulin      # Coverage analysis"
  echo "  cargo audit          # Security audit"
  echo "  cargo doc --open     # Generate and open docs"
  echo "  cargo expand         # Expand macros"
  echo "  cargo watch -c -x test  # Watch and test"
  echo ""
  echo "🐳 Container development:"
  echo "  podman-compose up -d # Start containerized services"
  echo "  drill --benchmark    # API stress testing"
  echo "  httpie/curl          # API testing"
  echo ""
''
