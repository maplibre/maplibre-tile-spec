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

`encode(geojson, name, extent=4096) -> bytes` encodes a single layer to an MLT blob.
`geojson` is a GeoJSON [`FeatureCollection`](https://datatracker.ietf.org/doc/html/rfc7946#section-3.3).
`name` and `extent` set the MLT layer metadata, since a `FeatureCollection` has no slot for them.
Geometry is in **tile-local coordinate space** (no projection), matching `tilezen/mapbox-vector-tile`'s default.
Coordinates must be integer-valued and 2D.
`extent` defaults to `4096`.

```python
import maplibre_tiles

blob = maplibre_tiles.encode(
    {
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "id": 1,
                "geometry": {"type": "Point", "coordinates": [2048, 1024]},
                "properties": {"name": "main", "lanes": 3},
            },
        ],
    },
    name="roads",
    extent=4096,
)

# Multi-layer tiles: encode each layer and concatenate the bytes
tile = b"".join([
    maplibre_tiles.encode(roads, name="roads"),
    maplibre_tiles.encode(water, name="water"),
])
```

Input is validated strictly.
A non-`FeatureCollection` input, a non-`Feature` member, non-integer or 3D coordinates, null or empty geometry, nested or non-scalar property values, a non-`u64` id, and an empty collection all raise `ValueError`.
