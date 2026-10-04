# MapLibre Tile Specification

> [!NOTE]
>
> This is a live specification that evolves continuously. Features marked as are under active development and may change in future versions. Stable features are those without experimental tags.

The wire format is versioned by the first byte of every tile.

| Version | Page | Status |
| --- | --- | --- |
| `0x01` | [MLT v1](<https://maplibre.org/maplibre-tile-spec/specification/v1/index.md>) | Stable. |
| `0x02` | [MLT v2](<https://maplibre.org/maplibre-tile-spec/specification/v2/index.md>) | Under development. |

Both versions share the [data model](<https://maplibre.org/maplibre-tile-spec/overview/index.md>) and the compression schemes in [Encoding Algorithms](<https://maplibre.org/maplibre-tile-spec/encodings/index.md>).
