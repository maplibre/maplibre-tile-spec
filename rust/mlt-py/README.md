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
