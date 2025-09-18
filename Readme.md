# PS2Suitcase

PS2Suitcase is a Rust toolchain for building, inspecting, and packaging PlayStation 2 save archives. The project focuses on a reliable PSU packer and modern graphical front ends so creators can prepare saves without juggling legacy utilities.

## Project goals

- Deliver a cross-platform desktop experience for organising PS2 save projects, editing metadata, and producing PSU archives.
- Provide a scriptable command-line packer that integrates cleanly with build pipelines.
- Maintain reusable PS2 file-format libraries that power both the CLI and GUI applications.

## Maintained components

- **`suitcase` (PS2Suitcase GUI):** Full-featured desktop app with project workspaces, live validation, icon/sys editors, and PSU export workflows.
- **`psu-packer` (CLI):** Standalone packer that reads `psu.toml`, regenerates `icon.sys` when requested, and writes deterministic `.psu` archives.
- **`psu-packer-gui`:** Lightweight GUI wrapper around the packer for quick folder selection and metadata edits when the full PS2Suitcase interface is unnecessary.
- **Libraries:** `ps2-filetypes` (parsers and writers for PSU, ICON, and TITLE files), `memcard` and `ps2-mcm` (memory-card utilities under active refactor), and shared UI macros.
- **Packaging tooling:** `xtask-build-app` bundles the PS2Suitcase GUI into a macOS `.app` structure with the correct resources.

## Feature highlights

- Multi-tab editors for `psu.toml`, `icon.sys`, `title.cfg`, and icon textures, with previews powered by `ps2-filetypes`.
- Folder tree with hot-reloading via filesystem watchers and validation messages that point directly to problematic assets.
- Wizards for generating ICON files and automatically applying metadata presets.
- Optional regeneration of `icon.sys` from structured TOML, ensuring consistent headers inside each archive.
- Ready-to-use templates located in `assets/templates/` for both configuration and metadata files.

## Build and run

### Prerequisites

- The latest stable Rust toolchain (install via [`rustup`](https://rustup.rs/)).
- On Linux, install system dependencies required by `wgpu` (Vulkan/GL drivers). On macOS, ensure the Xcode command-line tools are present.

### Command-line packer

```bash
# Run directly against a project directory containing psu.toml
cargo run -p psu-packer -- path/to/save-project

# Optional: choose the output path
cargo run -p psu-packer -- path/to/save-project -o output/ExampleSave.psu
```

Produce a release binary with:

```bash
cargo build -p psu-packer --release
```

The resulting executable lives in `target/release/psu-packer` (or `.exe` on Windows).

### Graphical interfaces

#### PS2Suitcase (full editor)

```bash
# Default build enables both the wgpu and glow renderers for portability
cargo run -p suitcase

# Build an optimised binary
cargo build -p suitcase --release
```

The release binary can be distributed from `target/release/suitcase`/`suitcase.exe`.

#### PSU Packer GUI (focused packer)

```bash
cargo run -p psu-packer-gui
```

This windowed utility mirrors the CLI packer with a simplified layout for quick packaging jobs.

## Platform support

- **Windows 10/11:** Native builds with either renderer; the build script embeds the application icon when compiling on Windows.
- **macOS (Intel & Apple Silicon):** Supported via Metal-backed `wgpu`. Use the packaging instructions below to create a signed `.app` bundle for distribution.
- **Linux (Wayland/X11):** Supported through `wgpu` (Vulkan/GL) or the `glow` fallback. Ensure GPU drivers expose the required APIs.
- **Command-line tools:** Build and run anywhere Rust targets (`psu-packer` does not depend on a windowing backend).

## Packaging PS2Suitcase for release

1. Build a release binary:
   ```bash
   cargo build -p suitcase --release
   ```
2. (Windows/Linux) Ship the `target/release/suitcase(.exe)` binary together with the `assets` directory if you need to provide templates or icons alongside the executable.
3. (macOS) After building, create an application bundle:
   ```bash
   cargo run -p xtask-build-app
   ```
   This writes `build/PSU Builder.app/` with the executable, icon, and `Info.plist`. Codesign and notarise as required for distribution.
4. Include sample templates from `assets/templates/` in your distribution package so users can scaffold new projects quickly.

## Configuration (`psu.toml`)

A valid project folder contains a `psu.toml` file that looks like:

```toml
[config]
name = "Example Save"
timestamp = "2024-10-10 10:30:00" # Optional (local time)
include = ["BOOT.ELF", "TITLE.DB"]  # Files to package
exclude = ["debug.log"]               # Optional inverse selector

[icon_sys]
flags = "PS2 Save File"               # Accepts numeric values or preset names
title = "Example Save"
linebreak_pos = 16                     # Optional, defaults to safe value
background_transparency = 0            # Optional

# Each list must contain the number of entries expected by the PS2 firmware
background_colors = [
    { r = 0, g = 32, b = 96, a = 0 },
    { r = 0, g = 48, b = 128, a = 0 },
    { r = 0, g = 64, b = 160, a = 0 },
    { r = 0, g = 16, b = 48, a = 0 },
]
light_directions = [
    { x = 0.0, y = 0.0, z = 1.0, w = 0.0 },
    { x = -0.5, y = -0.5, z = 0.5, w = 0.0 },
    { x = 0.5, y = -0.5, z = 0.5, w = 0.0 },
]
light_colors = [
    { r = 1.0, g = 1.0, b = 1.0, a = 1.0 },
    { r = 0.5, g = 0.5, b = 0.6, a = 1.0 },
    { r = 0.3, g = 0.3, b = 0.4, a = 1.0 },
]
ambient_color = { r = 0.2, g = 0.2, b = 0.2, a = 1.0 }
```

Key expectations:

- `name` sets the memory-card folder name and defaults the CLI output file name.
- Use either `include` *or* `exclude` to control file selection; leaving both empty packages every file.
- `timestamp` is optional. When omitted, the packer writes the current time.
- The `[icon_sys]` table is optional. When provided (or toggled on in the GUIs) the packer regenerates `icon.sys` from the supplied values and adds it to the archive automatically.
- Array lengths are validated; the packer reports descriptive errors if counts do not match firmware expectations.
- Templates live in `assets/templates/psu.toml` and `assets/templates/title.cfg`.

## Contributing

1. Fork and clone the repository.
2. Install the Rust toolchain and add `rustfmt` and `clippy` components (`rustup component add rustfmt clippy`).
3. Make your changes, then run:
   ```bash
   cargo fmt
   cargo clippy --all-targets --all-features
   cargo test --all-targets
   ```
4. Submit a pull request with a clear description of the change and relevant screenshots if you modified GUI behaviour.

Contributions covering new PS2 formats, validation improvements, or UI polish are especially welcome.

## License

PS2Suitcase is distributed under the MIT License. See [`LICENSE.txt`](LICENSE.txt) for the full text.

## Credits

Icon and UI design by [@Berion](https://www.psx-place.com/members/berion.1431/) and the PS2 homebrew community for test assets and documentation.
