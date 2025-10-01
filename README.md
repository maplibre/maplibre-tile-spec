# MapLibre Tiles (MLT)

The MapLibre Tile specification is mainly inspired by the [Mapbox Vector Tile (MVT)](https://github.com/mapbox/vector-tile-spec) specification,
but has been redesigned from the ground up to address the challenges of rapidly growing geospatial data volumes
and complex next-generation geospatial source formats as well as to leverage the capabilities of modern hardware and APIs.
MLT is specifically designed for modern and next generation graphics APIs to enable high-performance processing and rendering of
large (planet-scale) 2D and 2.5 basemaps. In particular, MLT offers the following features:
- **Improved compression ratio**: up to 6x on large tiles, based on a column oriented layout with recursively applied (custom)
    lightweight encodings. This leads to reduced latency, storage, and egress costs and, in particular, improved cache utilization
- **Better decoding performance**: fast lightweight encodings which can be used in combination with SIMD/vectorization instructions
- **Support for linear referencing and m-values** to efficiently support the upcoming next generation source formats such as Overture Maps (GeoParquet)
- **Support 3D coordinates**, i.e. elevation
- **Support complex types**, including nested properties, lists and maps
- **Improved processing performance**, based on storage and in-memory formats that are specifically designed for modern GL APIs,
allowing for efficient processing on both CPU and GPU. The formats are designed to be loaded into
GPU buffers with little or no additional processing

📝 For a more in-depth exploration of MLT have a look at the [following slides](https://github.com/mactrem/presentations/blob/main/FOSS4G_2024_Europe/FOSS4G_2024_Europe.pdf), watch
[this talk](https://www.youtube.com/watch?v=YHcoAFcsES0) or [read our documentation](https://maplibre.org/maplibre-tile-spec/).


> [!CAUTION]
> This format is still in the active research and development phase and is not ready for production use.

## Comparing with MVT

In the following, the size of MLT and MVT files are compared based on a selected set of representative tiles of an OpenMapTiles scheme based OSM dataset. While showing an up to factor 6 reduction in size on large tiles, initial tests also showed equal or mostly even better (between 2x and 3x even without the usage of SIMD) decoding performance compared to MVT in the browser.

| Zoom | Tile indices <br/>(minX,maxX,minY,maxY) | Tile Size Reduction |
|------|-----------------------------------------|---------------------|
| 2    | 2, 2, 2, 2                              | 53%                 |
| 3    | 4, 4, 5, 5                              | 48%                 |
| 4    |                                         | 80%                 |
| 5    | 16, 17, 20, 21                          | 81%                 |
| 6    | 32, 34, 41, 42                          | 76%                 |
| 7    | 66, 68, 83, 85                          | 74%                 |
| 8    | 132, 135, 170, 171                      | 74%                 |
| 9    | 264, 266, 340, 342                      | 68%                 |
| 10   | 530, 533, 682, 684                      | 60%                 |
| 12   | 2130, 2134, 2733, 2734                  | 65%                 |
| 13   | 4264, 4267, 5467, 5468                  | 50%                 |
| 14   | 8296, 8300, 10748, 10749                | 59%                 |

## Directory Structure

- `/spec` MLT specification and related documentation
- `/test` Test MVT tiles and the expected MLT conversion results
- `/java` Java encoder for converting MVT to MLT, as well as a decoder parsing MLT to an in-memory representation.
- `/js` Javascript decoder
- `/rust` Rust decoder

## Getting Involved

Join the #maplibre-tile-format slack channel at OSMUS: get an invite at https://slack.openstreetmap.us/

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
