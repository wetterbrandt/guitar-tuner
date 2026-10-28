# Polar Tuner

A native, completely offline electric guitar tuner for Windows and Linux. It provides a responsive needle mode for everyday tuning and a motion-based strobe display for precision setup and intonation work.

## Features

- Direct input from USB audio interfaces through WASAPI on Windows and ALSA/PipeWire on Linux
- Automatic preference for devices containing “Blackstar” or “Polar” in their name
- Selectable audio input at the bottom of the window
- YIN pitch detection across the guitar range (70–1,200 Hz)
- Fast needle mode and sub-cent strobe display
- Dark, minimalist native interface
- No network access, accounts, telemetry, or cloud services

## Run from source

Install the stable Rust toolchain. On Ubuntu, also install the native audio/window dependencies:

```sh
sudo apt install libasound2-dev libxkbcommon-dev
cargo run --release
```

Connect the guitar to the Blackstar Polar 2 and select it in the input selector. The app listens only; it does not alter interface output or direct monitoring.

## Build installers

Every push runs the GitHub Actions workflow and creates downloadable Windows `.exe`, Ubuntu/Debian `.deb`, and Fedora/Bazzite `.rpm` artifacts.

```sh
cargo install cargo-deb cargo-generate-rpm
cargo build --release
cargo deb --no-build
cargo generate-rpm
```

Runtime use is fully offline. The app never makes network requests.

## License

GPL-3.0-only. See [LICENSE](LICENSE).
