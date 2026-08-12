{
  description = "http-date: HTTP date parsing and formatting (RFC 9110)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Single source of truth for the stable toolchain: channel and
        # components are declared in rust-toolchain.toml.
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        # Nightly toolchain for cargo-fuzz: libFuzzer's sanitizers need the
        # nightly-only `-Zsanitizer`/`-Zbuild-std` flags. `rust-src` is
        # required for `-Zbuild-std`; `llvm-tools-preview` for coverage.
        nightly = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
          toolchain.minimal.override {
            extensions = [ "llvm-tools-preview" "rust-src" ];
          });
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "http-date";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          # Library crate: nothing to install, just create the output.
          installPhase = "mkdir -p $out";
        };

        devShells.default = pkgs.mkShell {
          packages = [
            toolchain
            pkgs.cargo-nextest
            pkgs.just
          ];

          # rust-analyzer needs the stdlib sources from the rust-src component.
          RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            echo "http-date dev shell: $(rustc --version) (rustfmt, clippy, rust-analyzer, nextest included)"
            echo "  fuzzing: nix develop .#fuzz  (or NIX_DEVSHELL=fuzz direnv reload)"
          '';
        };

        # Fuzzing environment: nightly toolchain + cargo-fuzz + clang.
        # Enter with `nix develop .#fuzz` (or `NIX_DEVSHELL=fuzz direnv reload`),
        # then run `cargo fuzz run decode` / `cargo fuzz run roundtrip`.
        devShells.fuzz = pkgs.mkShell {
          packages = [
            nightly
            pkgs.cargo-fuzz
            # libfuzzer-sys compiles libFuzzer from source and needs clang;
            # libcxx provides the C++ runtime it links against.
            pkgs.clang
            pkgs.llvmPackages.libcxx
          ];

          # rust-analyzer / cargo-fuzz `-Zbuild-std` source lookup.
          RUST_SRC_PATH = "${nightly}/lib/rustlib/src/rust/library";

          shellHook = ''
            echo "http-date fuzz shell: $(rustc --version) (cargo-fuzz, clang included)"
            echo "  try: cargo fuzz run roundtrip -- -runs=1000"
          '';
        };
      }
    );
}
