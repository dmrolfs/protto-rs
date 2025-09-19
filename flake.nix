{
  description = "Automagically convert prost types to your own types.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";

    # Add advisory-db for cargo-audit
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, advisory-db }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Enhanced rust toolchain with more components
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rustc-dev"
            "llvm-tools-preview"
            "rust-analyzer" # LSP support
          ] ++ pkgs.lib.optionals (!pkgs.stdenv.hostPlatform.isAarch64) [
            "miri" # Undefined behavior detection
          ];
          targets = [
            "wasm32-unknown-unknown" # WebAssembly support
            # Add other targets as needed
          ];
        };

        # Use crane with the rust-overlay rust version
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Source filtering for better cache efficiency
        src = craneLib.cleanCargoSource (craneLib.path ./.);

        # Extract crate info
        crateInfo = craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; };

        # Common arguments for all crane builds
        commonArgs = {
          inherit src;
          strictDeps = true;
          pname = "protto";
          version = "0.6.0";

          buildInputs = [
            # Runtime dependencies would go here
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # macOS-specific dependencies
            #            pkgs.darwin.apple_sdk.frameworks.Security
            #            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            pkgs.libiconv
          ];

          nativeBuildInputs = [
            pkgs.protobuf
            # Build-time dependencies
          ];

          # Environment variables
          PROTOC = "${pkgs.protobuf}/bin/protoc";
          PROTOC_INCLUDE = "${pkgs.protobuf}/include";

          # Improved build performance
          CARGO_PROFILE_RELEASE_LTO = "thin";
          CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "1";
        };

        # Build dependencies separately for better caching
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        workspaceBuild = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "--workspace --locked";

          # Generate documentation during build
          postBuild = ''
            cargo doc --workspace --no-deps
          '';
        });

        # Documentation build
        cargoDoc = craneLib.cargoDoc (commonArgs // {
          inherit cargoArtifacts;
          cargoDocExtraArgs = "--workspace --document-private-items";
        });

        # Development tools
        devTools = with pkgs; [
          # Core Rust tools
          rustToolchain
          rustfmt
          clippy

          # Protocol Buffers
          protobuf
          protoc-gen-rust

          # Testing and coverage
          cargo-tarpaulin
          cargo-nextest # Faster test runner
          cargo-mutants # Mutation testing

          # Security and auditing
          cargo-audit
          cargo-deny

          # Development utilities
          cargo-watch # File watching
          cargo-expand # Macro expansion
          cargo-machete # Unused dependency detection
          cargo-outdated # Dependency updates
          cargo-release # Release management

          # Git hooks and formatting
          pre-commit
          just # Command runner

          # Documentation
          mdbook # For additional documentation
        ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
          # Linux-only debugging tools
          #          gdb
          #          valgrind
          #          heaptrack
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          # macOS-specific debugging alternatives
          #          lldb                   # Apple's debugger
        ];

      in
      {
        packages = {
          default = workspaceBuild;
          doc = cargoDoc;
        };

        devShells = {
          # Default development shell
          default = craneLib.devShell {
            inputsFrom = [ workspaceBuild ];
            packages = devTools;
            #            packages = [ rustToolchain pkgs.protobuf ];

            shellHook = ''
              #              export LIBRARY_PATH="/System/Library/Frameworks:$LIBRARY_PATH"
              #              export CPATH="/System/Library/Frameworks:$CPATH"

                            echo "🦀 Rust development environment for protto"
                            echo "📦 Rust toolchain: $(rustc --version)"
                            echo "🔧 protoc version: $(protoc --version)"
                            echo ""
                            echo "Available commands:"
                            echo "  cargo build          # Build the project"
                            echo "  cargo test           # Run tests"
                            echo "  cargo nextest run    # Run tests with nextest"
                            echo "  cargo tarpaulin      # Coverage analysis"
                            echo "  cargo audit          # Security audit"
                            echo "  cargo doc --open     # Generate and open docs"
                            echo "  cargo expand         # Expand macros"
                            echo "  cargo watch -c -x test  # Watch and test"
                            echo ""

                            # Set up git hooks if pre-commit is available
                            if command -v pre-commit >/dev/null 2>&1; then
                              pre-commit install --install-hooks 2>/dev/null || true
                            fi

                            # Set RUST_SRC_PATH for IDE integration
                            export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
            '';
          };

          # Minimal shell for CI/automation
          ci = pkgs.mkShell {
            buildInputs = [
              rustToolchain
              pkgs.protobuf
              pkgs.cargo-tarpaulin
            ];
            PROTOC = "${pkgs.protobuf}/bin/protoc";
          };
        };

        # Comprehensive checks
        #      checks = {
        #        # Build checks
        #        workspace-build = craneLib.cargoBuild (commonArgs // {
        #          inherit cargoArtifacts;
        #          cargoExtraArgs = "--workspace --all-targets --locked";
        #        });
        #
        #        # Test checks
        #        workspace-test = craneLib.cargoNextest (commonArgs // {
        #          inherit cargoArtifacts;
        #          partitions = 1;
        #          partitionType = "count";
        #        });
        #
        #        # Formatting check
        #        fmtCheck = craneLib.cargoFmt (commonArgs // {
        #          inherit cargoArtifacts;
        #          cargoExtraArgs = "--all";
        #        });
        #
        #        clippyCheck = craneLib.cargoClippy (commonArgs // {
        #          inherit cargoArtifacts;
        #          cargoClippyExtraArgs = "--workspace --all-targets --locked -- -D warnings -W clippy::pedantic";
        #        });
        #
        #        # Documentation check
        #        doc-check = craneLib.cargoDoc (commonArgs // {
        #          inherit cargoArtifacts;
        #          cargoDocExtraArgs = "--workspace --document-private-items";
        #          RUSTDOCFLAGS = "-D warnings";
        #        });
        #
        #        # Security audit
        #        audit-check = craneLib.cargoAudit (commonArgs // {
        #          inherit advisory-db;
        #        });
        #
        #        # Coverage check (optional, can be slow)
        #        coverage-check = craneLib.cargoTarpaulin (commonArgs // {
        #          inherit cargoArtifacts;
        #          cargoTarpaulinExtraArgs = "--workspace --timeout 300 --out xml --output-dir coverage/";
        #        });
        #      };

        # Formatter for `nix fmt`
        formatter = pkgs.nixpkgs-fmt;

        # Apps that can be run with `nix run .#<name>`
        #      apps = {
        #        # Run coverage and open report
        #        coverage = flake-utils.lib.mkApp {
        #          drv = pkgs.writeShellScriptBin "coverage" ''
        #            ${pkgs.cargo-tarpaulin}/bin/cargo-tarpaulin tarpaulin --workspace --timeout 300 --out html --output-dir coverage/
        #            if command -v xdg-open >/dev/null 2>&1; then
        #              xdg-open coverage/tarpaulin-report.html
        #            elif command -v open >/dev/null 2>&1; then
        #              open coverage/tarpaulin-report.html
        #            fi
        #          '';
        #        };
        #
        #        # Run mutation testing
        #        mutants = flake-utils.lib.mkApp {
        #          drv = pkgs.writeShellScriptBin "mutants" ''
        #            ${pkgs.cargo-mutants}/bin/cargo-mutants mutants --in-place
        #          '';
        #        };
        #      };
      });
}
