# QGIS MLT Plugin

A QGIS plugin to open **MapLibre Tile (MLT)** files, powered by the Rust
`mlt` parser exposed to Python via [PyO3](https://pyo3.rs).

## Architecture

```
┌─────────────────────┐
│   QGIS Plugin       │  Python – registers menu action, creates memory layers
│   (qgis/mlt_plugin) │
└────────┬────────────┘
         │ import
┌────────▼────────────┐
│   mlt               │  Rust → Python bridge (PyO3 + maturin)
│   (rust/mlt-py)     │
└────────┬────────────┘
         │ depends on
┌────────▼────────────┐
│   mlt               │  Rust – zero-copy MLT binary parser
│   (rust/mlt)        │
└─────────────────────┘
```

## Prerequisites

- **QGIS 3.22+** (tested with 3.34)
- **Rust toolchain** (1.87+) — install via [rustup](https://rustup.rs)
- **maturin** — PyO3 build tool (`pip install maturin`)
- **Python 3.9+** (must match the Python that QGIS uses)

## Installation

### Step 1: Find QGIS's Python interpreter

The `mlt` native module must be built for the same Python that QGIS uses.

```bash
# Linux (system Python, usually the same one QGIS links to)
/usr/bin/python3 -c "import qgis.core; print('QGIS bindings OK')"

# macOS (QGIS bundles its own Python)
/Applications/QGIS.app/Contents/MacOS/bin/python3 --version
```

Use whichever interpreter prints "QGIS bindings OK" in the commands below.

### Step 2: Build and install the native module

```bash
cd rust/mlt-py

# Option A: build a wheel, then install it
pip install maturin            # in any environment
maturin build --release --interpreter /usr/bin/python3

# Install into user site-packages (visible to QGIS)
/usr/bin/python3 -m pip install --user --break-system-packages \
    ../../rust/target/wheels/mlt-*.whl

# Option B: if QGIS uses a virtualenv / conda, activate it first
#   conda activate qgis-env   # or: source /path/to/venv/bin/activate
#   maturin develop --release
```

Verify:

```bash
/usr/bin/python3 -c "import mlt; print(mlt.list_layers(open('../../test/synthetic/0x01/polygon.mlt','rb').read()))"
# Expected: ['layer1']
```

### Step 3: Symlink the plugin into QGIS

```bash
# Linux
ln -sfn "$(pwd)/../../qgis/mlt_plugin" \
    ~/.local/share/QGIS/QGIS3/profiles/default/python/plugins/mlt_plugin

# macOS
ln -sfn "$(pwd)/../../qgis/mlt_plugin" \
    ~/Library/Application\ Support/QGIS/QGIS3/profiles/default/python/plugins/mlt_plugin

# Windows (PowerShell, run as Admin)
# New-Item -ItemType SymbolicLink `
#   -Path "$env:APPDATA\QGIS\QGIS3\profiles\default\python\plugins\mlt_plugin" `
#   -Target (Resolve-Path "..\..\qgis\mlt_plugin")
```

### Step 4: Enable in QGIS

1. Launch QGIS
2. **Plugins → Manage and Install Plugins**
3. Search for **MLT Provider**, check the box to enable it
4. A toolbar icon and menu entry appear under **Vector → MLT Provider → Open MLT File(s)…**

## Usage

### Opening a single tile

1. Click **Open MLT File(s)…**
2. Select one `.mlt` file
3. A dialog appears with auto-detected **z/x/y** coordinates (parsed from the
   filename, e.g. `14_8297_10749.mlt` → z=14, x=8297, y=10749)
4. Confirm or edit the values, toggle **TMS y-axis** if needed
5. Click **OK** — each MLT layer loads as a QGIS memory layer with real
   EPSG:3857 coordinates

### Opening multiple tiles at once

1. Click **Open MLT File(s)…**
2. Select multiple `.mlt` files (Ctrl+click / Shift+click)
3. A table dialog shows all files with their auto-detected z/x/y
4. Options:
   - **TMS y-axis** — checked by default (correct for OpenMapTiles / MBTiles)
   - **Merge same-named layers** — checked by default; combines features from
     the same MLT layer across tiles into one QGIS layer for seamless viewing
5. Click **OK** — e.g. selecting a 3×3 tile grid creates unified "building",
   "transportation", "water" layers containing features from all 9 tiles

### Coordinate conventions

| Convention | y=0 is at | Used by |
|---|---|---|
| **TMS** (default) | South | OpenMapTiles, MBTiles, TileJSON |
| **XYZ** | North | OSM raster tile servers |

If coordinates look wrong (features in the ocean), try toggling TMS.

Click **Skip (raw coords)** to load tile-local integer coordinates without
geo-referencing (useful for inspecting raw tile data).

## Python API (standalone, without QGIS)

```python
import mlt

data = open("tile.mlt", "rb").read()

# Structured decode with geo-referencing
layers = mlt.decode_mlt(data, z=14, x=8297, y=10749, tms=True)
for layer in layers:
    print(f"Layer: {layer.name}, extent: {layer.extent}")
    for feat in layer.features[:3]:
        print(f"  id={feat.id}, type={feat.geometry_type}")
        print(f"  wkb={len(feat.wkb)} bytes, props={dict(feat.properties)}")

# Raw tile-local coordinates (no z/x/y needed)
layers = mlt.decode_mlt(data)

# GeoJSON string output (tile-local coords)
geojson_str = mlt.decode_mlt_to_geojson(data)

# Fast layer listing (no full decode)
names = mlt.list_layers(data)
```

## Troubleshooting

**"mlt module not found"** — the native module isn't installed for QGIS's
Python. Re-run Step 2 using the correct interpreter.

**Features appear in the ocean** — toggle the TMS checkbox, or verify z/x/y
values are correct.

**Plugin not visible in QGIS** — check the symlink points to the right
directory and that the `metadata.txt` file exists inside it.

**Build fails with "unsafe_code forbidden"** — the `mlt-py` crate overrides
the workspace lint locally; make sure you're building from `rust/mlt-py/`,
not the workspace root.
