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

`encode(layer, options=None) -> bytes` encodes a single layer to an MLT blob.
Geometry is in **tile-local coordinate space** (no projection), matching
`tilezen/mapbox-vector-tile`'s default. Coordinates must be integer-valued and 2D.

`layer` is either a layer dict (`{name, extent?, features}`) or a GeoJSON
`FeatureCollection` (its `name`/`extent` come from `options`). Each feature's
`geometry` may be a GeoJSON geometry dict, a WKT string, or WKB bytes.

```python
import maplibre_tiles

blob = maplibre_tiles.encode(
    {
        "name": "roads",
        "extent": 4096,
        "features": [
            # GeoJSON geometry dict
            {"id": 1, "geometry": {"type": "Point", "coordinates": [2048, 1024]},
             "properties": {"name": "main", "lanes": 3}},
            # WKT string
            {"id": 2, "geometry": "LINESTRING (0 0, 10 0, 10 10)"},
            # WKB bytes
            {"id": 3, "geometry": b"\x01\x01\x00\x00\x00..."},
        ],
    },
    {"sort": "auto", "tessellate": False},  # options (all optional)
)

# A GeoJSON FeatureCollection works too; name/extent come from options
blob = maplibre_tiles.encode(feature_collection, {"name": "roads", "extent": 4096})

# Multi-layer tiles: encode each layer and concatenate the bytes
tile = b"".join([
    maplibre_tiles.encode(roads_layer),
    maplibre_tiles.encode(water_layer),
])
```

Options (all optional):

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| `extent` | `int` | `4096` | Tile extent (coordinate space is `[0, extent)`). |
| `name` | `str` | — | Layer name. Required when `layer` is a `FeatureCollection`; ignored if the layer dict already has a `name`. |
| `tessellate` | `bool` | `False` | Generate polygon tessellation data. |
| `sort` | `"auto"` \| `"morton"` \| `"hilbert"` \| `"id"` \| `"none"` | `"auto"` | Feature sort strategy. `"auto"` tries all and keeps the smallest; `"none"` preserves input order. |
| `allow_fsst` | `bool` | `True` | Allow FSST string compression. |
| `allow_fpf` | `bool` | `True` | Allow FastPFOR integer compression. |
| `allow_shared_dict` | `bool` | `True` | Allow grouping strings into shared dictionaries. |

Input is validated strictly: non-integer or 3D coordinates, null/empty geometry,
nested/non-scalar property values, non-`u64` ids, and empty layers all raise
`ValueError`.
