{
  description = "exec_and_record - record terminal commands to video and logs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "clippy" "rustfmt" "rust-src" ];
        };

        execAndRecordPkg = pkgs.callPackage ./package.nix { };
      in
      {
        packages.exec_and_record = execAndRecordPkg;
        packages.default = execAndRecordPkg;

        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain

            # Cargo dev tools
            pkgs.cargo-nextest
            pkgs.cargo-deny
            pkgs.cargo-llvm-cov
            pkgs.cargo-mutants
            pkgs.cargo-insta

            # Runtime tools
            pkgs.asciinema
            pkgs.agg
            pkgs.ffmpeg
            pkgs.util-linux
          ];
        };
      }
    );
}
