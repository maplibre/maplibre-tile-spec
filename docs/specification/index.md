<h1>MapLibre Tile Specification</h1>

--8<-- "live-spec-note"

The wire format is versioned by the first byte of every tile.

| Version | Page | Status |
|---------|------|--------|
| `0x01` | [MLT v1](v1.md) | Stable. |
| `0x02` | [MLT v2](v2.md) | <span class="experimental"></span> Under development. |

Both versions share the data model in the [Overview](../overview.md) and the compression schemes in [Encoding Algorithms](../encodings.md).
