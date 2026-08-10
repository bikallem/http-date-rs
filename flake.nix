{
  description = "http-date-rs: HTTP date parsing and formatting (RFC 9110)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Single source of truth for the toolchain: channel and components
        # are declared in rust-toolchain.toml.
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "http-date-rs";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          # Library crate: nothing to install, just create the output.
          installPhase = "mkdir -p $out";
        };

        devShells.default = pkgs.mkShell {
          packages = [ toolchain ];

          # rust-analyzer needs the stdlib sources from the rust-src component.
          RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            echo "http-date-rs dev shell: $(rustc --version) (rustfmt, clippy, rust-analyzer included)"
          '';
        };
      });
}
