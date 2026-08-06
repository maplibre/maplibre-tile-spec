# mlt_py

Python bindings for the MapLibre Tile (MLT) format via [PyO3](https://pyo3.rs/).

## Decoding

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


## Decoded objects


`decode_mlt(...)` returns a list of `MltLayer` objects.
Each `MltLayer` represents one decoded MLT layer and exposes:
- `name: str` — the layer name.
- `extent: int` — the layer extent.
- `features: list[MltFeature]` — the decoded features in that layer.

Each `MltFeature` represents one decoded feature and exposes:
- `id: int | None` — the feature id, if present.
- `geometry_type: str` — the decoded geometry type.
- `wkb: bytes` — the geometry as WKB.
- `properties: dict` — the feature properties.


```python
import maplibre_tiles


layers = maplibre_tiles.decode_mlt(data)
for layer in layers:
    print(layer.name, layer.extent)
    for feature in layer.features[:2]:
        print(feature.id, feature.geometry_type, dict(feature.properties))
```


## Encoding


`encode_geojson(geojson, name, extent=4096, *, tessellate=False, sort="auto", shared_dict=True, fsst=True, fastpfor=True) -> bytes` encodes a single layer to an MLT blob.
- `geojson` is a GeoJSON [`FeatureCollection`](https://datatracker.ietf.org/doc/html/rfc7946#section-3.3).
  Geometry is in **tile-local coordinate space** (no projection), matching `tilezen/mapbox-vector-tile`'s default.
  Coordinates must be integers and 2D.
  They must be JSON integers (`2048`), not floats: a float-typed value such as `2048.0` raises `ValueError`.
- `name` and `extent` set the MLT layer metadata, since a `FeatureCollection` has no slot for them.
  `extent` defaults to `4096`.
- `tessellate` generates triangulation data for polygons and multi-polygons.
- `sort` controls which feature ordering(s) the encoder tries: `all`, `auto`, `morton`, `hilbert`, `id`, or `none`.
- `shared_dict` enables grouping strings into shared dictionaries.
- `fsst` enables FSST string compression.
- `fastpfor` enables FastPFOR integer compression.

`encode_mvt(data, *, tessellate=False, sort="auto", shared_dict=True, fsst=True, fastpfor=True) -> bytes` encodes an entire raw Mapbox Vector Tile (protobuf) to MLT using the same encoding options.


```python
import maplibre_tiles


blob = maplibre_tiles.encode_geojson(
    {
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "id": 1,
                "geometry": {"type": "Point", "coordinates": },
                "properties": {"name": "main", "lanes": 3},
            },
        ],
    },
    name="roads",
    extent=4096,
)


# Multi-layer tiles -> encode each layer and concatenate the bytes
tile = b"".join([
    maplibre_tiles.encode_geojson(roads, name="roads"),
    maplibre_tiles.encode_geojson(water, name="water"),
])
```


```python
import maplibre_tiles


mvt = open("tile.mvt", "rb").read()

blob = maplibre_tiles.encode_mvt(mvt)


# With explicit encoding options
blob = maplibre_tiles.encode_mvt(
    mvt,
    tessellate=False,
    sort="auto",
    shared_dict=True,
    fsst=True,
    fastpfor=True,
)
```

Input is validated strictly.
A non-`FeatureCollection` input, a non-`Feature` member, float-typed or 3D coordinates, null or empty geometry, nested or non-scalar property values, a non-`u64` id, and an empty collection all raise `ValueError`.
