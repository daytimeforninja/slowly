{
  description = "Coherence Breathing - A Cosmic desktop app for breathing training";

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
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        buildInputs = with pkgs; [
          # Wayland and display
          wayland
          libxkbcommon
          libGL
          mesa

          # Font rendering
          fontconfig
          freetype
          expat

          # Additional X11 support (some Cosmic apps need this)
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi

          # Vulkan (optional but recommended)
          vulkan-loader

          # Audio
          alsa-lib
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          rustToolchain
          cargo
        ];

        # Runtime library path
        libPath = pkgs.lib.makeLibraryPath buildInputs;

      in {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

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
