# Contributing to Aiden

Thank you for considering a contribution to Aiden.

Aiden is a local-first biosignal computer-control project. Contributions should preserve predictable behavior, explicit safety boundaries, deterministic signal recognition, and clear separation between the UI and timing-sensitive native processing.

## Before Opening an Issue

For bugs:

1. Search existing issues first.
2. Use the bug-report form.
3. Include Aiden version, OS, display server where relevant, signal source, and clear reproduction steps.
4. Include diagnostics only after reviewing them for private paths or personal information.
5. Do not publish biosignal recordings or medical information unless you intentionally want them to be public.

For security problems, do not create a public issue. Follow `SECURITY.md`.

## Development Requirements

Aiden currently uses:

- Rust
- Tauri 2
- React
- TypeScript
- Vite
- React Flow
- SQLite
- Arduino Uno firmware

Node.js 22 and a current stable Rust toolchain are recommended for development.

## Windows Setup

Install:

- Node.js 22
- Rust with the MSVC toolchain
- Visual Studio C++ Build Tools
- Windows SDK
- Microsoft Edge WebView2 Runtime

Then:

```powershell
npm.cmd ci
rustup component add rustfmt clippy
npm.cmd run tauri -- dev
```

## Linux Setup

The current native input implementation targets X11.

On Ubuntu/Linux Mint:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  pkg-config \
  curl \
  ca-certificates \
  git \
  libwebkit2gtk-4.1-dev \
  libudev-dev \
  libxdo-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf \
  xdg-utils
```

Then install Node.js 22 and stable Rust:

```bash
npm ci
rustup component add rustfmt clippy
npm run tauri -- dev
```

## Validation Before a Pull Request

Frontend:

```bash
npm test
npm run build
```

Rust:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check

cargo clippy --locked \
  --manifest-path src-tauri/Cargo.toml \
  --all-targets \
  -- -D warnings

cargo test --locked \
  --manifest-path src-tauri/Cargo.toml \
  --all-targets
```

If your change affects firmware, compile the Uno sketch with Arduino IDE or Arduino CLI and state what you tested.

## Pull Request Guidelines

Keep pull requests focused.

A pull request should explain:

- what changed
- why the change is needed
- how it was tested
- which platforms were tested
- whether hardware testing was performed
- whether persistence or database behavior changed
- whether computer-control behavior changed
- whether any safety behavior changed

Do not claim physical-hardware validation if the change was only tested in simulation.

## Architectural Principles

### Keep real-time work native

High-frequency serial acquisition, DSP, recognition, graph execution, persistence, and operating-system control belong in the Rust backend.

The React frontend should not become responsible for timing-sensitive recognition or automation.

### Keep recognition deterministic

Aiden currently does not use AI or machine-learning classifiers for Action Profile recognition.

Do not introduce:

- neural-network inference
- LLM dependencies
- cloud classifiers
- required cloud processing
- hidden telemetry

without an explicit project-level design decision.

### Preserve safety behavior

Changes must not silently enable live computer control.

Important safety mechanisms include:

- monitoring off by default
- Test Graph mode
- explicit live-control confirmation
- emergency stop
- release of held input state
- approval of sensitive system targets

Changes affecting these areas require careful tests.

### Do not make medical claims

Aiden is not a medical device.

Documentation and UI text must not describe signal values as diagnostic health information.

## Hardware Contributions

New biosensor or acquisition-device support should be introduced behind clean interfaces rather than hard-coding new hardware assumptions throughout the application.

Document:

- electrical interface
- sampling characteristics
- protocol
- tested operating systems
- tested hardware revision
- known limitations

## Style

Prefer:

- small focused modules
- explicit error handling
- bounded queues/buffers
- deterministic behavior
- tests for signal-processing and graph changes
- clear comments where behavior is non-obvious

Avoid unrelated formatting changes in functional pull requests.

## Attribution

Aiden was created by:

**Sam (17ofSeptember)**  
https://github.com/17ofSeptember  
awrynetwork@gmail.com
