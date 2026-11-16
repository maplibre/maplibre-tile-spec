## CLI Tool

The `mlt` binary provides several commands for working with MLT files:

### Commands

* **`dump`** - Parse an MLT file and dump raw layer data without decoding
* **`decode`** - Parse an MLT file, decode all layers, and dump the result (supports text and `GeoJSON` output)
* **`hexdump`** - Annotated byte/bit-level hexdump of an MLT file's metadata and stream payloads
* **`convert`** - Convert MVT or MLT tile files and MVT `.mbtiles`/`.pmtiles` archives to MLT, or tile a `GeoJSON` file, including 3D positions, or `CityGML` buildings into MLT files
* **`ui`** - Interactive terminal visualizer for MLT files

### Format conversion

Convert an MVT archive or files to MLT:

```bash
mlt convert input.mvt.pmtiles output.mlt.pmtiles
```

`--mlt-version 2` writes the v2 format instead of v1, the default.
v2 is unstable and `MapLibre` renderers cannot read it yet.

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

### Tiling GeoJSON

Give `convert` a WGS84 `GeoJSON` `FeatureCollection` to tile it into a `z/x/y.mlt` directory
as a single layer named after the file. `--max-zoom` is required, and `--min-zoom` and
`--layer` are optional; the encoder options above apply as to any other input. This writes
the v2 format, so it needs `--mlt-version 2`:

```bash
mlt convert buildings.geojson tiles --mlt-version 2 --max-zoom 14
```

Positions with an altitude (`[lon, lat, alt]`) become z coordinates on a `--z-step` grid,
a power of ten in metres from `-3` (1 mm) to `4` (10 km):

```bash
mlt convert power_lines.geojson tiles --mlt-version 2 --max-zoom 14 --z-step -1
```

An altitude is metres above mean sea level unless `--height-mode relative-to-ground` makes it metres above the terrain.

### Tiling buildings

`convert` also tiles the buildings of a `CityGML` 2.0 or 3.0 `.gml` file, or of a directory of
them, into a directory of `z/x/y.mlt` tiles with one layer of the `Meshes` layout of
[`docs/specification/v2.md`](../../docs/specification/v2.md#meshes). Like a `GeoJSON` input, it
needs `--mlt-version 2` and `--max-zoom`, and takes `--min-zoom` (0), `--layer` (the input's file
name) and `--z-step`:

```bash
mlt convert citygml/hamburg tiles --mlt-version 2 --min-zoom 14 --max-zoom 16
```

Each building, bridge or tunnel becomes one feature per zoom, reprojected from the file's
`srsName` (with vertical units scaled to metres). A building is stored whole, in the tile of each
zoom that holds the centre of its Web Mercator `(x, y)` bounding box. Its z values are heights
above its ground datum (`GroundSurface` at that centre, or its lowest vertex without one), in the
`relativeToGround` height mode and on a 1 cm grid unless `--z-step` says otherwise, and each
triangle carries the `part` m-value (wall, roof, ground, window or door), so surfaces of different
parts share their vertices. Each feature has the building's `gml_id` property, and `--verify`
round-trip checks every encoded mesh tile.

Lower zooms match simplification error to screen pixel size, a pixel being a 512th of a tile:

- `--max-zoom` holds the highest LOD of each part of a building, simplified within 5 cm along part
  seams.
- One zoom below holds LOD2 (`>= 1.5` pixels across): a part's own LOD2 or LOD1 if the file has
  one, or else its highest LOD without windows, doors, installations and the holes they leave, its
  vertices clustered in cells of half a pixel and simplified as a whole within half a pixel, each
  triangle then a roof, ground or wall by its facing.
- Two zooms below keeps coarse 3D LOD2 (1-pixel cells, 0.75-pixel error) on buildings at least
  14 m tall or 6 pixels across so pitched roofs and spires transition smoothly, while smaller
  buildings (`>= 2.5` pixels across) become extruded ground-outline blocks.
- Three or more zooms below merge the blocks of a tile's buildings under 40 m where they touch:
  the cells of a pixel that their grounds cover at least half of are joined with their neighbours
  in the same 10 m band of tops and bases, and each group becomes one block at its mean top. Its
  outline is simplified to within a pixel, and it has no wall where it stands less than a pixel
  above its neighbour or the ground. Such a block has no `gml_id`, but a `buildings` property
  counting the buildings it holds. Landmarks of 40 m or more keep their simplified 3D LOD2 mesh
  across all lower zooms, whatever their footprint size.

With `--fix-mislabelled-crs`, a file whose coordinates lie more than 1 degree from the input's
other files in the CRS it declares, but near them in the CRS most files declare, is read in the
latter with a warning: some Hamburg files declare EPSG:25833 for UTM zone 32 coordinates.

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
  - Every panel scrolls and shows a scrollbar when its content does not fit
  - The tree also scrolls sideways with `<`/`>`, `Shift+Left`/`Shift+Right`, or a sideways mouse scroll
  - Hovered rows are highlighted with underlined green text
- **Map Panel (right)**: Visual representation of the geometries
  - Shows the extent boundary as a thin gray rectangle
  - Draws the selected level in color and the rest in gray as context
  - Tessellation edges stored in the tile are drawn in dark gray inside the selected polygon, or inside a hovered one in a layer
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
