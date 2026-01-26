<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://maplibre.org/img/maplibre-logos/maplibre-logo-for-dark-bg.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://maplibre.org/img/maplibre-logos/maplibre-logo-for-light-bg.svg">
    <img alt="MapLibre Logo" src="https://maplibre.org/img/maplibre-logos/maplibre-logo-for-light-bg.svg" width="200">
  </picture>
</p>

# MapLibre Tile (MLT)

> [!NOTE]
> [The specification](https://maplibre.org/maplibre-tile-spec/specification/) is deemed stable as of October 2025. However, as a living standard, experimental features may continue to evolve. Implementations and integrations are being developed, refer to the [implementation status](https://maplibre.org/maplibre-tile-spec/implementation-status/) page for the current status.

The MapLibre Tile specification is mainly inspired by the [Mapbox Vector Tile (MVT)](https://github.com/mapbox/vector-tile-spec) specification,
but has been redesigned from the ground up to address the challenges of rapidly growing geospatial data volumes
and complex next-generation geospatial source formats as well as to leverage the capabilities of modern hardware and APIs.
MLT is specifically designed for modern and next generation graphics APIs to enable high-performance processing and rendering of
large (planet-scale) 2D and 2.5 basemaps. In particular, MLT offers the following features:
- **Improved compression ratio**: up to 6x on large encoded tiles, based on a column oriented layout with recursively applied (custom)
    lightweight encodings. This leads to reduced latency, storage, and egress costs and, in particular, improved cache utilization
- **Better decoding performance**: fast lightweight encodings which can be used in combination with SIMD/vectorization instructions
- **Support for linear referencing and m-values** to efficiently support the upcoming next generation source formats such as Overture Maps (GeoParquet)
- **Support 3D coordinates**, i.e. elevation
- **Support complex types**, including nested properties, lists and maps
- **Improved processing performance**, based on storage and in-memory formats that are specifically designed for modern GL APIs,
allowing for efficient processing on both CPU and GPU. The formats are designed to be loaded into
GPU buffers with little or no additional processing

📝 For a more in-depth exploration of MLT have a look at the [following slides](https://github.com/mactrem/presentations/blob/main/FOSS4G_2024_Europe/FOSS4G_2024_Europe.pdf), watch
[this talk](https://www.youtube.com/watch?v=YHcoAFcsES0) or read [this paper](https://dl.acm.org/doi/10.1145/3748636.3763208) by MLT inventor Markus Tremmel.

## Directory Structure

- `/spec` MLT specification and related documentation
- `/test` Test MVT tiles and the expected MLT conversion results
- `/java` Java encoder for converting MVT to MLT, as well as a decoder parsing MLT to an in-memory representation.
- `/js` Javascript decoder
- `/rust` Rust decoder

## Getting Involved

Join the `#maplibre-tile-format` Slack channel at OSM US: get an invite at https://slack.openstreetmap.us/

### Development

* This project is easier to develop with [just](https://just.systems/man/en/), a modern alternative to `make`.
* To get a list of available commands, run `just`.
* To run tests, use `just test`.

## License

* All project documentation and specification content is licensed under [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/) - Public Domain Dedication.
* All code is dual licensed under [Apache License v2.0](http://www.apache.org/licenses/LICENSE-2.0) and [MIT license](http://opensource.org/licenses/MIT), at your option.
* Tile test data in the `/test` directory has different licenses depending on the source of that data.

### Contribution

Unless you explicitly state otherwise, any code contribution intentionally
submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions. Similarly, any documentation or specification
contributions shall be licensed under CC0 1.0 Universal.

## Citing

If you use MapLibre Tile in your research, please cite our paper:

```bibtex
@inproceedings{tremmel2025maplibretile,
  title     = {MapLibre Tile: A Next Generation Vector Tile Format},
  author    = {Tremmel, Markus and Zink, Roland},
  booktitle = {Proceedings of the 33rd ACM International Conference on Advances in Geographic Information Systems},
  series    = {SIGSPATIAL '25},
  year      = {2025},
  pages     = {1118--1121},
  doi       = {10.1145/3748636.3763208},
  url       = {https://doi.org/10.1145/3748636.3763208}
}
```

