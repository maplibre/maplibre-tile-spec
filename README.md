# MapLibre Tiles (MLT)

The MLT format is mainly inspired by the [Mapbox Vector Tile (MVT)](https://github.com/mapbox/vector-tile-spec) specification, but has been redesigned from the ground
up to primarily improve on the following topics:
- **Improved compression ratio**: up to 6x on large tiles, based on a column oriented layout with (custom) lightweight encodings
- **Better decoding performance**: fast lightweight encodings which can be used in combination with SIMD/vectorization instructions
- **Support for linear referencing and m-values**: to efficiently support the upcoming next generation source formats such as Overture Maps (GeoParquet)
- **Support for a more complex type system**: Including nested properties, lists and maps
- **Support for 3D coordinates**
- **Improved processing performance**: Based on an in-memory format that can be processed efficiently on the CPU and GPU and loaded directly
  into GPU buffers partially (like polygons in WebGL) or completely (in case of WebGPU compute shader usage) without additional processing


## Comparison with Mapbox Vector Tiles

In the following, the size of MLT and MVT files are compared based on a selected set of representative tiles of an 
OpenMapTiles scheme based OSM dataset. While showing an up to factor 6 reduction in size on large tiles, initial tests also 
showed equal or mostly even better (between 2x and 3x even without the usage of SIMD) decoding performance compared 
to MVT in the browser.


| Zoom level | Tile indices <br/>(minX,maxX,minY,maxY) | Tile Size Reduction (%) | 
|------------|-----------------------------------------|----------------------|
| 2          | 2,2,2,2                                 | 53                   |
| 3          | 4,4,5,5                                 | 48                   |
| 4          |                                         | 80                   |
| 5          | 16,17,20,21                             | 81                   |
| 6          | 32,34,41,42                             | 76                   |
| 7          | 66,68,83,85                             | 74                   |
| 8          | 132, 135, 170, 171                      | 74                   |
| 9          | 264, 266, 340, 342                      | 68                   |
| 10         | 530, 533, 682, 684                      | 60                   |
| 12         | 2130, 2134, 2733, 2734                  | 65                   | 
| 13         | 4264, 4267, 5467, 5468                  | 50                   |
| 14         | 8296, 8300, 10748, 10749                | 59                   |

## Information about the repository

> **_Notice:_** This repository is still in the active research and development phase and is not ready for production use.  
It contains experimental research code that is subject to change.

This repository has the following directory structure:
- ``spec``: Contains a rough first (yet incomplete specified) draft of the MLT specification
- ``converter``: Contains a POC Java encoder for converting MVT to MLT files as well as decoder for converting 
  the MLT storage into the in-memory format.
- ``test``: Contains a set of test tilesets.
- ``parser``: A POC web based decoder for an earlier MLT version, which demonstrated 
  that MLT, in addition to a significant reduced tile size, also shows a equal or mostly even better
  (between 2x and 3x even without the usage of SIMD) decoding performance compared to MVT.
  Since the previous focus was on the Java POC encoder and decoder, the js decoder is not 
  yet updated to the newest changes in the specification, which will cause all unit tests to fail.
- ``evaluaiton``: Different benchmarks and evaluations for finding the best encodings and file layout.

