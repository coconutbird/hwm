# hwm — Halo Wars Manager

A command-line tool for managing **Halo Wars** installations and mods.

It detects your game install, stores per-game configuration, swaps the active
mod in and out, and launches the game — plus inspects mod packages.

> **Status:** Halo Wars: Definitive Edition (`hw1`) is fully supported.
> Halo Wars 2 (`hw2`) is scaffolded throughout but detection and launch are not
> yet implemented. **Windows only** (uses the registry and `%LOCALAPPDATA%`).

## Install

Build from source with a recent Rust toolchain (edition 2024):

```sh
cargo build --release
```

The binary is produced at `target/release/hwm`.

> The build pulls the `era` / `xmb` crates from
> [`ensemble-formats`](https://github.com/coconutbird/ensemble-formats) as git
> dependencies, so the first build needs access to that repository.

## Usage

```
hwm <command>
```

Games are referred to as `hw1` (Halo Wars: Definitive Edition) or `hw2`.

### Configure a game

```sh
# Auto-detect the install (HW1, via the Steam registry entry) and save it
hwm config path hw1 --detect

# Or set the path manually
hwm config path hw1 "C:\Games\Halo Wars Definitive Edition"

# Show the current path / clear it
hwm config path hw1
hwm config path hw1 --clear

# Show configuration for all games
hwm config show
```

Configuration is stored as `config.toml` next to the executable.

### Set the active mod

```sh
# Point the game at a mod directory
hwm config mod-path hw1 "C:\Mods\MyMod"

# Show / clear the configured mod
hwm config mod-path hw1
hwm config mod-path hw1 --clear
```

### Launch

```sh
# Launch with the configured mod active
hwm launch hw1

# Launch vanilla (clears the mod for this run)
hwm launch hw1 --vanilla
```

On launch, `hwm` writes the active mod path to
`%LOCALAPPDATA%\Halo Wars\ModManifest.txt` (empty for vanilla), then starts the
game executable.

### Inspect a mod

```sh
# A mod folder (contains a .hwmod manifest + ModData/)
hwm mod info ./MyMod

# Or the .hwmod manifest file directly
hwm mod info "./My Mod v1.0.hwmod"
```

`mod info` reports the title/author/version, whether the declared `ModID`
matches its computed hash, whether the banner/icon art and `ModData/` folder are
present, and the resolved manifest path.

## Mod format

`hwm` reads the **hwmod** format (Firebase / HaloWarsModding). A mod is a
**folder** containing:

- a `*.hwmod` **manifest** — an XML file (the `.hwmod` extension *is* the
  manifest; it is not an archive), and
- a `ModData/` folder with the actual game content.

```
MyMod/
├── My Mod v1.0.hwmod   # XML manifest
└── ModData/            # game content
```

Art paths are relative to the manifest's folder, and `ModID` is the uppercase
SHA-256 of `<title-author-version>` — so `hwm` can flag a manifest whose ID
doesn't match its metadata. Example manifest:

```xml
<HWMod ManifestVersion="1" ModID="...">
  <RequiredData Title="My Mod" Author="Me" Version="1.0" />
  <OptionalData>
    <BannerArt>art/banner.png</BannerArt>
    <Icon>art/icon.png</Icon>
    <Description>What this mod does.</Description>
  </OptionalData>
</HWMod>
```

Only `RequiredData` (Title / Author / Version) is mandatory.

## Development

```sh
cargo test      # run the test suite
cargo clippy    # lint
```

## License

Not yet specified.
