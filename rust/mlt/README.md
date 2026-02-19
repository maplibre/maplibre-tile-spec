## CLI Tool

The `mlt` binary provides several commands for working with MLT files:

### Commands

* **`dump`** - Parse an MLT file and dump raw layer data without decoding
* **`decode`** - Parse an MLT file, decode all layers, and dump the result (supports text and `GeoJSON` output)
* **`ui`** - Interactive terminal visualizer for MLT files

### Visualizer

The visualizer command provides an interactive terminal-based UI for exploring MLT files:

```bash
# Visualize a single MLT file
cargo run -- ui path/to/file.mlt

# Browse and visualize all MLT files in a directory (recursive)
cargo run -- ui path/to/directory
```

**Directory Mode**:
- Lists all `.mlt` files found recursively in the directory
- Use `↑`/`↓` to navigate the file list
- Press `Enter` to open and visualize a file
- Press `Esc` to go back to file list
- Press `q` to quit

Features:
- **Tree View Panel (left)**: Browse layers and features in a hierarchical tree
- "All Layers" - shows all features from all layers
  - Individual layers - shows all features in that layer
  - Individual features - shows only the selected feature
  - Hovered features are highlighted with underlined green text
- **Map Panel (right)**: Visual representation of the geometries
  - Shows the extent boundary as a thin gray rectangle
  - **Color coding by geometry type**:
    - Points: Magenta (multi-point: light magenta)
    - `LineStrings`: Cyan (multi-linestring: light cyan)
    - Polygons: Blue/Red based on winding order (multi-polygon: same)
  - **Polygon winding order visualization**:
    - Blue: Counter-clockwise rings (typically outer rings)
    - Red: Clockwise rings (typically holes)
  - Selected features: Yellow
  - Hovered features: White
  - Automatically adjusts bounds to fit all visible geometries
- **Mouse Interaction**:
  - Hover over geometries to highlight them in the tree view
- **Keyboard Navigation**:
  - `↑`/`k` - Move selection up
  - `↓`/`j` - Move selection down
  - `Enter` - In layer overview mode, switch to detail mode; In file browser, open selected file
  - `Esc` - Go back (detail → overview → file list) or quit if at top level
  - `q` - Quit the visualizer
