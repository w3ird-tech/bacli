{
  description = "Packages and development environment for the Bitaxe CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          let
            pkgs = import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            };
            rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          in
          f pkgs rust
        );
    in
    {
      packages = forAllSystems (
        pkgs: rust:
        let
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rust;
            rustc = rust;
          };
          manifest = builtins.fromTOML (builtins.readFile ./crates/bacli/Cargo.toml);
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./crates
            ];
          };
          mkPackage =
            buildType:
            rustPlatform.buildRustPackage {
              pname = "bacli";
              version = manifest.package.version;
              inherit src buildType;
              cargoLock.lockFile = ./Cargo.lock;
              cargoBuildFlags = [
                "--package"
                "bacli"
              ];
              cargoTestFlags = [
                "--workspace"
                "--all-features"
              ];
              nativeBuildInputs = [
                pkgs.pkg-config
                pkgs.cmake
              ];
              buildInputs = [
                pkgs.openssl
                pkgs.aws-lc
              ];
              strictDeps = true;
              dontStrip = buildType == "debug";
              meta = {
                description = manifest.package.description;
                homepage = manifest.package.repository;
                license = pkgs.lib.licenses.agpl3Only;
                mainProgram = "bacli";
              };
            };
        in
        rec {
          dev = mkPackage "debug";
          release = mkPackage "release";
          default = release;
        }
      );

      devShells = forAllSystems (
        pkgs: rust:
        let
          nightlyRustfmt = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.rustfmt);
        in
        {
          default = pkgs.mkShell {
            packages = [
              rust
              pkgs.just
              pkgs.cargo-machete
              pkgs.cargo-dist
            ];
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.cmake
            ];
            buildInputs = [ pkgs.openssl ];

            # cargo fmt uses nightly's import grouping without a rustup installation.
            RUSTFMT = "${nightlyRustfmt}/bin/rustfmt";
          };
        }
      );
    };
}
