{
  description = "USVFS Bindings for the Rust Programming Language";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, fenix, naersk, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        target = "x86_64-pc-windows-gnu";

        toolchain = with fenix.packages.${system}; combine [
          minimal.cargo
          minimal.rustc
          targets.${target}.latest.rust-std
        ];

        naersk' = naersk.lib.${system}.override {
          cargo = toolchain;
          rustc = toolchain;
        };
      in rec {
        defaultPackage = packages.x86_64-pc-windows-gnu;

        packages.x86_64-pc-windows-gnu = naersk'.buildPackage {
          src = ./.;
          strctDeps = true;
        };

        depsBuildBuild = with pkgs; [
          pkgsCross.mingwW64.stdenv.cc
          pkgsCross.mingwW64.windows.pthreads
        ];

        nativeBuildInputs = with pkgs; [
          wineWowPackages.stable
        ];

        doCheck = true;

        CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";

        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER = pkgs.writeScript "wine-wrapper" ''
          export WINEPREFIX="$(mktemp -d)"
          exec wine 64 $@
        '';

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            pkgsCross.mingwW64.stdenv.cc
            pkgsCross.mingwW64.windows.mingw_w64_pthreads
            pkgsCross.mingwW64.windows.pthreads
            wineWowPackages.stable
            rust-analyzer
            toolchain
          ];

          shellHook = ''
            echo Rust!
          '';
        };
      });
}
