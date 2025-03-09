{
  description = "Automagically convert prost types to your own types.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        craneLib = crane.mkLib pkgs;

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustc-dev" "llvm-tools-preview" ];
        };

        src = craneLib.path ./.;
        crateInfo = craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; };

        commonArgs = {
          inherit src;
          strictDeps = true;
          pname = crateInfo.pname;
          version = crateInfo.version;
          nativeBuildInputs = [ rustToolchain pkgs.protobuf ];
          PROTOC = "${pkgs.protobuf}/bin/protoc";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        workspaceBuild = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "--workspace";
        });
      in
      {
        packages.default = workspaceBuild;

        devShells.default = craneLib.devShell {
          inputsFrom = [ workspaceBuild ];
          packages = with pkgs; [
            rustToolchain
            rustfmt
            clippy
            protobuf
            cargo-tarpaulin
          ];
          shellHook = ''
          '';
        };

        checks = {
          fmtCheck = craneLib.cargoFmt (commonArgs // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--all";
          });

          clippyCheck = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--workspace --all-targets -- -D warnings";
          });

          buildCheck = craneLib.cargoBuild (commonArgs // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--workspace --all-targets";
          });
        };
      });
}
