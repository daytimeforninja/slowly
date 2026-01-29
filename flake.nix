{
  description = "Coherence Breathing - A Cosmic desktop app for breathing training";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        buildInputs = with pkgs; [
          wayland
          libxkbcommon
          libGL
          mesa
          fontconfig
          freetype
          expat
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          vulkan-loader
          alsa-lib
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        libPath = pkgs.lib.makeLibraryPath buildInputs;

        src = craneLib.cleanCargoSource ./.;

        cargoArtifacts = craneLib.buildDepsOnly {
          inherit src buildInputs nativeBuildInputs;
          strictDeps = true;
        };

        slowly = craneLib.buildPackage {
          inherit src cargoArtifacts buildInputs nativeBuildInputs;
          strictDeps = true;
          postFixup = ''
            patchelf --set-rpath "${libPath}" $out/bin/slowly
          '';
        };

      in {
        packages.default = slowly;

        devShells.default = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs = nativeBuildInputs ++ [ rustToolchain ];

          shellHook = ''
            export LD_LIBRARY_PATH="${libPath}:$LD_LIBRARY_PATH"
            export RUST_BACKTRACE=1
            echo "Coherence Breathing dev environment ready!"
            echo "Run 'cargo build' to compile, 'cargo run' to launch"
          '';
        };
      }
    );
}
