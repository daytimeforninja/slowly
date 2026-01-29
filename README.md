# Slowly

Coherence breathing trainer for the [COSMIC](https://system76.com/cosmic) desktop.

An expanding circle guides your breath through inhale and exhale phases while optional audio cues keep you on rhythm. Configurable timing presets target different physiological responses — from HRV-optimal coherence breathing to calming or energizing patterns.

## Features

- **Breathing presets** — Coherence (5s/5s), Relaxation (4s/8s), Energizing (4s/2s), Square (4s/4s), or custom timing
- **Animated circle** — Expands on inhale, contracts on exhale with particle effects
- **Audio** — Brown noise, binaural beats, and phase transition tones (independently togglable)
- **Session timer** — 5/10/15/20/30 minutes or infinite
- **Statistics** — Tracks total practice time, sessions completed, and daily practice
- **Persistent settings** — Configuration and stats saved to `~/.config/slowly/slowly/settings.json`

## Install

### Nix flake

```nix
# flake input
slowly.url = "github:daytimeforninja/slowly";

# add to packages
inputs.slowly.packages.${system}.default
```

### From source

```sh
nix develop  # or install deps manually: wayland, libxkbcommon, libGL, fontconfig, alsa-lib, pkg-config
cargo run
```

## License

[Hippocratic License 3.0](LICENSE.md)
