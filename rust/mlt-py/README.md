# mlt_py

Python bindings for the MapLibre Tile (MLT) format via [PyO3](https://pyo3.rs/).

```python
import maplibre_tiles

data = open("tile.mlt", "rb").read()

# Structured decode with geo-referencing
layers = maplibre_tiles.decode_mlt(data, z=14, x=8297, y=10749, tms=True)
for layer in layers:
    print(f"Layer: {layer.name}, extent: {layer.extent}")
    for feat in layer.features[:3]:
        print(f"  id={feat.id}, type={feat.geometry_type}")
        print(f"  wkb={len(feat.wkb)} bytes, props={dict(feat.properties)}")

# Raw tile-local coordinates (no z/x/y needed)
layers = maplibre_tiles.decode_mlt(data)

# GeoJSON string output (tile-local coords)
geojson_str = maplibre_tiles.decode_mlt_to_geojson(data)

# Fast layer listing (no full decode)
names = maplibre_tiles.list_layers(data)
```

## Encoding

`encode(layer) -> bytes` encodes a single layer to an MLT blob.
Geometry is in **tile-local coordinate space** (no projection), matching
`tilezen/mapbox-vector-tile`'s default. Coordinates must be integer-valued and 2D.

`layer` is a layer dict (`{name, extent?, features}`). Each feature's `geometry`
is a GeoJSON geometry dict. `extent` defaults to `4096` if omitted.

```python
import maplibre_tiles

blob = maplibre_tiles.encode(
    {
        "name": "roads",
        "extent": 4096,
        "features": [
            {"id": 1, "geometry": {"type": "Point", "coordinates": [2048, 1024]},
             "properties": {"name": "main", "lanes": 3}},
        ],
    },
)

# Multi-layer tiles: encode each layer and concatenate the bytes
tile = b"".join([
    maplibre_tiles.encode(roads_layer),
    maplibre_tiles.encode(water_layer),
])
```

Input is validated strictly: non-integer or 3D coordinates, null/empty geometry,
nested/non-scalar property values, non-`u64` ids, and empty layers all raise
`ValueError`.
