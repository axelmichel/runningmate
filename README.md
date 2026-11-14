![Logo](./logo-runningmate.png)
# RunningMate

![GitHub release](https://img.shields.io/github/v/release/axelmichel/runningmate?include_prereleases)

RunningMate is a desktop app to store, analyse and visualise your running, cycling and walking
activities. It's built with [Rust](https://www.rust-lang.org) and [Tauri](https://tauri.app), with
a React interface using [shadcn/ui](https://ui.shadcn.com).

The app does not store any of your data outside your computer. Everything lives in the
`RunningData` folder in your home directory: a [SQLite](https://www.sqlite.org/) database, the
archived TCX files (`imports`) and your photos and videos (`media`).

## Documentation
The documentation can be found [here](https://axelmichel.github.io/runningmate).

## Features
- **Sync** activities in from TCX files, a watched folder (e.g. an iCloud export of RunGap or
  HealthFit) or **Garmin Connect** — Garmin works both ways: new Garmin activities are imported,
  activities recorded elsewhere can be uploaded.
- Weather for each activity (also historical), from [Open-Meteo](https://open-meteo.com).
- Search and filter activities; a heatmap of training time over the last year.
- Route, pace and heart-rate maps, elevation/heart-rate/pace charts, per-km segments and best
  segments.
- Heart-rate zones, training effect, fitness/fatigue trend.
- Photos and videos per activity, picked straight from the Photos library on macOS, dropped onto
  the app or linked from disk.
- A route image (PNG/SVG, white on transparent) to share or lay over photos.
- English and German.

## Installation
Download the latest release from the
[releases page](https://github.com/axelmichel/runningmate/releases) and open the installer for your
system (on macOS the `.dmg`). The app isn't code-signed yet, so macOS asks for confirmation the
first time you open it.

## Development
Prerequisites: [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 22 with pnpm 9,
and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your system.

```sh
cd app
pnpm install
pnpm tauri dev     # run the app in development mode
pnpm tauri build   # build the installer
```

To work against a copy of your data, point the app at another folder:
`RUNNINGMATE_DATA_DIR=/path/to/copy pnpm tauri dev`.

### Checks
All pull requests run these:

```sh
cd app
pnpm typecheck && pnpm lint && pnpm format:check && pnpm test
cd src-tauri
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

See [`app/README.md`](app/README.md) for the layout of the code.

### Documentation
The documentation is written in Markdown in the `docs` folder and built with
[MkDocs](https://www.mkdocs.org/) (`pip install mkdocs mkdocs-material`, then `mkdocs serve`).
Changes on `main` are published to GitHub Pages automatically.

## Contributing
Feel free to contribute by submitting issues or pull requests on GitHub. Please follow the existing
style and include documentation for new features.
