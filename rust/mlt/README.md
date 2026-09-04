## CLI Tool

The `mlt` binary provides several commands for working with MLT files:

### Commands

* **`dump`** - Parse an MLT file and dump raw layer data without decoding
* **`decode`** - Parse an MLT file, decode all layers, and dump the result (supports text and `GeoJSON` output)
* **`hexdump`** - Annotated byte/bit-level hexdump of an MLT file's metadata and stream payloads
* **`convert`** - Convert MVT or MLT tile files and MVT `.mbtiles`/`.pmtiles` archives to MLT
* **`ui`** - Interactive terminal visualizer for MLT files

### Format conversion

Convert an MVT archive or files to MLT:

```bash
mlt convert input.mvt.pmtiles output.mlt.pmtiles
```

The conversion summary reports unique, decompressed tile payloads as such:

```text
input.mvt.pmtiles -> output.mlt.pmtiles (pmtiles):
  converted 5 tiles (5 unique encoded, 0 cache hits) in 740.9ms
  size raw/archive: MVT(gzip) 813.7kB/459.8kB -> MLT(gzip) 460.3kB/357.0kB
```

### Partial conversion

Pass `--bbox min_lon,min_lat,max_lon,max_lat` to convert one region of an archive,
the same way `martin-cp` limits what it copies:

```bash
mlt convert planet.mvt.pmtiles berlin.mlt.pmtiles --bbox 13.0,52.3,13.8,52.7
```

Every tile overlapping the bounds is converted, at every zoom level, and the output's
bounds and center metadata shrink to the region. Repeat `--bbox` to keep several regions.
It needs an `.mbtiles` or `.pmtiles` input, the only ones that record where each tile is.

### Visualizer

The visualizer command provides an interactive terminal-based UI for exploring MLT files:

```bash
# Visualize a single MLT file
cargo run -- ui path/to/file.mlt

# Browse and visualize all MLT files in a directory (recursive)
cargo run -- ui path/to/directory
```

**Directory Mode**:
- Lists every `.mlt`, `.mvt`, and `.pbf` file under the directory; the walk and the per-file analysis run in the background, so the list fills in while you browse
- Use `↑`/`↓` to navigate the file list; the preview, filter, and info panels follow the selection
- Click a column header to sort once the scan is done; click the filter checkboxes to narrow the list
- Press `Enter` to open and visualize a file
- Press `Esc` to go back to file list
- Press `q` to quit

Features:
- **Tree View Panel (left)**: Browse layers and features in a hierarchical tree
  - "All" - every layer, with layer and feature counts and the tile's geometry types
  - Individual layers - the layer's features, with each property's value types and the layer's geometry types
  - Individual features - the feature's properties and geometry statistics
  - Hovered rows are highlighted with underlined green text
- **Map Panel (right)**: Visual representation of the geometries
  - Shows the extent boundary as a thin gray rectangle
  - Draws the selected level in color and the rest in gray as context
  - Tessellation triangles stored in the tile are drawn in gray behind the selected polygon, or behind a hovered one inside a layer
  - **Color coding by geometry type**:
    - Points: Magenta (multipoint: light magenta)
    - `LineStrings`: Cyan (multi-linestring: light cyan)
    - Polygons: Blue/Red based on winding order (multi-polygon: same)
  - **Polygon winding order visualization**:
    - Blue: Counter-clockwise rings (typically outer rings)
    - Red: Clockwise rings (typically holes)
  - Selected features: Yellow
  - Hovered features: White
  - Automatically adjusts bounds to fit all visible geometries
- **Mouse Interaction**:
  - Hovering the map targets one level below the selection, so a layer under All, a feature under a layer, a part under a feature
  - The hovered row is underlined and the info panels describe it
  - Clicking the map selects the hovered item
- **Keyboard Navigation**:
  - `↑`/`k` - Move selection up
  - `↓`/`j` - Move selection down
  - `Enter` - In layer overview mode, switch to detail mode; In file browser, open selected file
  - `Esc` - Go back (detail -> overview -> file list) or quit if at top level
  - `q` - Quit the visualizer
