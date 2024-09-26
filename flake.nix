{
  description = "rust dev shell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url  = "github:numtide/flake-utils";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.inputs.flake-utils.follows = "flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      with pkgs;
      {
        formatter = pkgs.alejandra;

        devShells.default = mkShell {
          buildInputs = [
            ((rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
              extensions = [ "rust-src" ];
            })
            # rust-analyzer
            pkg-config
          ];

          # RUSTFLAGS = (builtins.map (a: ''-L ${a}/lib'') [
          #   # add libraries here (e.g. pkgs.libvmi)
          # ]);
        };
      }
    );
}