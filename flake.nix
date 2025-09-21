{
  description = "Comprehensive Rust development environment with containerization tools and modern CLI utilities";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";

    # Add advisory-db for cargo-audit
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, fenix, advisory-db }:
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

        # DMR: Make Crane usage configurable
        useCrane = true; # Set to false to use traditional cargo builds

        # DMR: Crane setup (when enabled)
        craneLib = if useCrane then (crane.mkLib pkgs).overrideToolchain rustToolchain else null;

        # Modern CLI tools
        modernCliTools = with pkgs; [
          # Modern replacements for core utilities
          ripgrep # rg - better grep
          fd # fd - better find
          bat # bat - better cat with syntax highlighting
          eza # eza - better ls (successor to exa)
          procs # procs - better ps
          dust # dust - better du
          htop # htop - better top
          tokei # tokei - code statistics
          bandwhich # bandwhich - network utilization by process
          gping # gping - ping with graph
          hyperfine # hyperfine - command-line benchmarking
          starship # starship - cross-shell prompt

          # DMR: Log analysis and monitoring tools
          lnav # lnav - log file navigator and analyzer
          # loki-cli       # DMR: Alternative modern log viewer (uncomment if preferred)
        ];

        # Development tools
        devTools = with pkgs; [
          # Core Rust tools
          rustToolchain
          rustfmt
          clippy

          # Protocol Buffers
          protobuf
          protoc-gen-rust

          # Container tools
          podman
          podman-compose

          # Task runner and build tools
          just # Command runner
          meson # Build system
          ninja # Build system
          pkg-config
          openssl

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

          # Documentation
          mdbook # For additional documentation

          # Network and API tools
          curl
          jq
          httpie

          # MinIO client for debugging/management
          minio-client

          # API stress testing
          drill

          # Database and lakehouse tools
          sqlite # Local database for development

        ] ++ modernCliTools ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
          # Linux-only debugging tools
          gdb
          valgrind
          # heaptrack  # Uncomment if needed
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          # macOS-specific dependencies
          pkgs.libiconv
          # macOS-specific debugging alternatives
          # lldb       # Apple's debugger
        ];

        # DMR: Reference external starship configuration file
        starshipConfig = ./nix-config/starship.toml;

        # DMR: Reference external shell aliases configuration
        shellAliasesConfig = import ./nix-config/shell-aliases.nix;

        # DMR: Reference external environment configuration (general app development)
        commonEnvVars = import ./nix-config/env-vars.nix { inherit pkgs; };

        # DMR: Reference external common shell hook
        commonShellHook = import ./nix-config/shell-hook.nix { inherit pkgs rustToolchain; };

        # DMR: Common arguments for builds (both crane and traditional)
        commonArgs = {
          strictDeps = true;

          buildInputs = with pkgs; [
            # Runtime dependencies would go here
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # macOS-specific dependencies
            pkgs.libiconv
          ];

          nativeBuildInputs = with pkgs; [
            protobuf
            pkg-config
            openssl
          ];

          # Environment variables for builds
          PROTOC = "${pkgs.protobuf}/bin/protoc";
          PROTOC_INCLUDE = "${pkgs.protobuf}/include";
          CARGO_PROFILE_RELEASE_LTO = "thin";
          CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "1";
        } // (if useCrane then {
          src = craneLib.cleanCargoSource (craneLib.path ./.);
        } else { });

        # Build dependencies separately for better caching
        cargoArtifacts = if useCrane then craneLib.buildDepsOnly commonArgs else null;

        # DMR: Conditional Crane build artifacts and packages
        cranePackages =
          if useCrane then {
            # Main build
            workspaceBuild = craneLib.buildPackage (commonArgs // {
              inherit cargoArtifacts;
              cargoExtraArgs = "--workspace --locked";
              postBuild = ''
                cargo doc --workspace --no-deps
              '';
            });

            # Documentation build
            cargoDoc = craneLib.cargoDoc (commonArgs // {
              inherit cargoArtifacts;
              cargoDocExtraArgs = "--workspace --document-private-items";
            });

            # Comprehensive checks
            checks = {
              workspace-build = craneLib.cargoBuild (commonArgs // {
                inherit cargoArtifacts;
                cargoExtraArgs = "--workspace --all-targets --locked";
              });

              workspace-test = craneLib.cargoNextest (commonArgs // {
                inherit cargoArtifacts;
                partitions = 1;
                partitionType = "count";
              });

              fmtCheck = craneLib.cargoFmt (commonArgs // {
                inherit cargoArtifacts;
                cargoExtraArgs = "--all";
              });

              clippyCheck = craneLib.cargoClippy (commonArgs // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--workspace --all-targets --locked -- -D warnings -W clippy::pedantic";
              });

              doc-check = craneLib.cargoDoc (commonArgs // {
                inherit cargoArtifacts;
                cargoDocExtraArgs = "--workspace --document-private-items";
                RUSTDOCFLAGS = "-D warnings";
              });

              audit-check = craneLib.cargoAudit (commonArgs // {
                inherit advisory-db;
              });

              coverage-check = craneLib.cargoTarpaulin (commonArgs // {
                inherit cargoArtifacts;
                cargoTarpaulinExtraArgs = "--workspace --timeout 300 --out xml --output-dir coverage/";
              });
            };
          } else { };

      in
      {
        # Multiple development shells for different purposes
        devShells = {
          # Default comprehensive development shell
          default = pkgs.mkShell
            {
              buildInputs = devTools;
              shellHook = commonShellHook;
            } // commonEnvVars;

          # Minimal shell for CI/automation
          ci = pkgs.mkShell
            {
              buildInputs = with pkgs; [
                rustToolchain
                protobuf
                pkg-config
                openssl
                cargo-tarpaulin
                cargo-nextest
              ];

              shellHook = ''
                echo "🤖 CI Environment Ready"
                export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
              '';
            } // commonEnvVars;

          # DMR: Focused on containerization and API development tools
          containerdev = pkgs.mkShell
            {
              buildInputs = with pkgs; [
                rustToolchain
                podman
                podman-compose
                just
                drill
                curl
                jq
                httpie
                protobuf
                pkg-config
                openssl
              ] ++ modernCliTools;

              shellHook = ''
                eval "$(starship init bash)"
                echo "🐳 Container Development Environment"
                echo "Available: podman, podman-compose, just, drill, modern CLI tools"
                echo "Perfect for: API development, microservices, containerized applications"
              '';
            } // commonEnvVars // { shellAliases = shellAliasesConfig; };

          # DMR: Enhanced crane shell with crane-specific features
          crane =
            if useCrane then craneLib.devShell
              {
                inputsFrom = [ cranePackages.workspaceBuild ];
                packages = devTools;
                shellHook = commonShellHook + ''
                  echo "🏗️  Crane Build System Active"
                  echo "  nix build          # Build with Crane (cached layers)"
                  echo "  nix build .#doc    # Generate documentation"
                  echo "  nix flake check    # Run all Crane checks"
                '';
              } // commonEnvVars else pkgs.mkShell
              {
                buildInputs = devTools;
                shellHook = commonShellHook + ''
                  echo "🏗️  Crane Build System (Disabled)"
                  echo "  Set useCrane = true in flake.nix to enable"
                '';
              } // commonEnvVars;
        };

        # Packages (can be used by projects that include this flake)
        packages = {
          # Export the rust toolchain
          rust-toolchain = rustToolchain;

          # Export development tools bundle
          dev-tools = pkgs.symlinkJoin {
            name = "rust-dev-tools";
            paths = devTools;
          };
        } // (if useCrane then {
          # DMR: Crane-built packages
          default = cranePackages.workspaceBuild;
          doc = cranePackages.cargoDoc;
          workspace = cranePackages.workspaceBuild;
        } else {
          # DMR: Traditional cargo-based packages (empty, use cargo directly)
        });

        # Formatter for `nix fmt`
        formatter = pkgs.nixpkgs-fmt;

        # DMR: Conditional checks (only when using Crane)
        checks = if useCrane then cranePackages.checks else { };

        # Apps that can be run with `nix run .#<n>`
        apps = {
          # Starship setup
          setup-starship = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "setup-starship" ''
              echo "📦 Setting up Starship configuration..."
              mkdir -p ~/.config
              cp ${starshipConfig} ~/.config/starship.toml
              echo "✅ Starship configuration installed to ~/.config/starship.toml"
              echo "Add 'eval \"\$(starship init bash)\"' to your shell rc file if not using nix develop"
            '';
          };

          # Pre-commit setup
          setup-precommit = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "setup-precommit" ''
              echo "📦 Setting up pre-commit hooks..."
              if [ -f .pre-commit-config.yaml ]; then
                ${pkgs.pre-commit}/bin/pre-commit install --install-hooks
                echo "✅ Pre-commit hooks installed"
              else
                echo "❌ No .pre-commit-config.yaml found"
                echo "Create one with: nix run .#create-precommit-config"
              fi
            '';
          };

          # DMR: Setup project structure and copy template files
          create-precommit-config = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "create-precommit-config" ''
              echo "📝 Creating .pre-commit-config.yaml from template..."
              if [ -f nix-config/pre-commit-template.yaml ]; then
                cp nix-config/pre-commit-template.yaml .pre-commit-config.yaml
                echo "✅ .pre-commit-config.yaml created from template"
              else
                echo "❌ Template not found at nix-config/pre-commit-template.yaml"
                exit 1
              fi
              echo "Run: nix run .#setup-precommit to install hooks"
            '';
          };
        };

        # Export commonly used configurations
        lib = {
          inherit commonArgs commonEnvVars;
          inherit rustToolchain modernCliTools devTools;
          shellAliases = shellAliasesConfig;
        };
      });
}
