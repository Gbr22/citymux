{
  description = "A devShell with a rust toolchain for cross-compiling to windows";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url  = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          crossSystem = {
            config = "x86_64-w64-mingw32";
          };
          overlays = [
            (import rust-overlay)
          ];
        };

        pkgsLocal = import nixpkgs {
          inherit system;
        };
        rust-toolchain = pkgs.pkgsBuildHost.rust-bin.stable.latest.default.override {
          targets = [ "x86_64-pc-windows-gnu" "x86_64-unknown-linux-gnu" ];
        };
      in
      with pkgs;
      {
        packages.default = rustPlatform.buildRustPackage {
          name = "citymux";
          src = builtins.path {
            path = ./.;
            name = "sile";
            filter = path: type:
              let
                ignoreFiles = [
                  "flake.nix"
                  "flake.lock"
                  "default.nix"
                  "shell.nix"
                  ".gitignore"
                  ".git"
                ];
              in
              # If the `path` checked is found in ignoreFiles, don't add it to the source
              if pkgs.lib.lists.any (p: p == (baseNameOf path)) ignoreFiles then
                false
              else
                true
              ;
          };
          cargoDeps = rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };
          buildInputs = [
            pkgs.windows.mingw_w64_pthreads
          ];
          nativeBuildInputs = [
            rust-toolchain
            pkgsLocal.wine64
          ];
          buildPhase = ''
            cargo build --release --target x86_64-pc-windows-gnu
            cargo build --release --target x86_64-unknown-linux-gnu
          '';
          installPhase = ''
            mkdir -p $out/bin
            cp ./target/x86_64-pc-windows-gnu/release/citymux.exe $out/bin/x86_64-pc-windows-gnu.exe
            cp ./target/x86_64-unknown-linux-gnu/release/citymux $out/bin/x86_64-unknown-linux-gnu.bin
          '';
        };
        devShells.default = mkShell {
          buildInputs = [
            pkgs.windows.mingw_w64_pthreads
          ];
          nativeBuildInputs = [
            rust-toolchain
            pkgsLocal.wine64
          ];
        };
      }
    );
}