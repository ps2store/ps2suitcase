**PSU Packer GUI overview:** The application presents a straightforward window for selecting PSU source folders, configuring metadata, and packing archives without needing to use the command line.

***This fork of the original project simply intends to add a simple gui to the existing progress of the project. Nothing fancy or pretty, but should work good for those who don't like to use terminals.***
*This fork only addresses the PSU packer, and not the other featured programs of the repo. Only windows has been tested.*

---------------------------------------------------------------------
# ORIGINAL PROJECT BY https://github.com/ps2store/ps2suitcase & https://github.com/techwritescode & https://github.com/mcoirault
# PS2 Rust

Monorepo of [tech's] Rust projects for PS2 homebrew.

## Crates

### ps2-filetypes

A collection of PS2 file type parsers.

### ps2-mcm

Memory Card Manager

### psu-packer-gui

Graphical interface for packing PSU archives. Run with:

```
cargo run -p psu-packer-gui
```

### Configuring icon.sys metadata

`psu-packer` now understands an optional `[icon_sys]` table inside `psu.toml`. When the table is present—or when the GUI toggle is enabled—the packer regenerates `icon.sys` before packaging and automatically includes it even if the file is missing from the `include` list.

#### Example `psu.toml`

```toml
[config]
name = "Example Save"
timestamp = "2024-10-10 10:30:00"
include = ["BOOT.ELF", "TITLE.DB"]

[icon_sys]
flags = "PS2 Save File"
title = "Example Save"
background_transparency = 0
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

* `flags` accepts either a numeric value or one of the descriptive names from the GUI drop-down (for example `"PS2 Save File"`, `"Software (PS2)"`, or `"System Driver"`).
* `background_colors` must contain exactly four entries; `light_directions` and `light_colors` expect three entries each. Omit these arrays to keep the defaults.
* `ambient_color` and `background_transparency` are optional—leave them out to rely on the standard lighting values bundled with the packer.

### Templates

Starter templates for both configuration files live in `assets/templates/psu.toml` and `assets/templates/title.cfg`. The GUI `File` menu offers "Create … from template" actions so you can drop these starting points straight into a project before editing.

The GUI mirrors the same settings so you can preview and persist changes without touching the TOML by hand.

## Credits

Icon & UI Design by [@Berion](https://www.psx-place.com/members/berion.1431/)
