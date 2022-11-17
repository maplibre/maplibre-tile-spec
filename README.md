## Cloud Optimized Vector Tiles (COVTiles)

The main idea of COVTiles is to use a column oriented layout like the state of the art big data formats Parquet and ORC 
or the in-memory format Apache Arrow instead of the row oriented approach on which Mapbox Vector Tiles (MVT) is based on. 
This enables the usage of custom encodings on a specific column like RLE and Bit-Vector encoding 
for example in addition to a tile wide data agnostic compression like Gzip or Brotli. 
This can significantly improve the compression ratio. 
For improving the decoding performance SIMD instructions will be used as 
for example described in the paper “Decoding billions of integers per second through vectorization” 
or "Lightweight Data Compression Algorithms: An Experimental Survey".
In addition the tiles should be "cloud optimized" which means that also only specific layer of a tile can be queried
via HTTP range reqeusts.

The main goals of COVTiles are:
- Minimize the tile size and thereby the network latency -> currently up to 7x in some zoom levels -> 10x possible?
- Faster decoding through vectorization
- Making the tile "cloud optimized" so that also specific layers of a tile can be queried via HTTP range requests

## General

### Mapbox Vector Tiles (MVT)
[Mapbox Vector Tiles (MVT)](https://github.com/mapbox/vector-tile-spec) are the state of the art way 
of storing and visualizing planet scale spatial vector tilesets.
They are optimized in size through the usage of a local tile coordinate system, delta and Base128 Varint encoding 
for the geometries as well as dictionary encoding for the keys and values of the feature properties.

### Limitations

For example discussed in https://github.com/mapbox/vector-tile-spec/issues/35

## Main Concepts

### Column oriented

### Custom compression

Based on the layout of COVTiles a custom compression can be used per column.

#### Geometry

The information about the geometry of a feature is separated in a ``geometryType``, ``topology`` and ``vertexBuffer`` column.
Using separate columns for describing the geometry of a feature enables a better optimization of the compression.

#### GeometryType

The geometryType column encodes one of the ``OGC Simple Feature Access Model`` specified geometry types.
A layer can have different geometry types, but as most layer contain only geometries of a single type 
RLE encoding combined with bitpacking (bitwidth 3) significantly reduces the size.

#### Topology
The ``topology`` column describes the structure of the geometry.
The informations to describe the topology of a geometry is stored in separate ``streams``.
This has the advantage that the streams can have distinct encodings.
Depending on the type of the geometry the topology column has the following structure:
- LineString Streams -> numVertices
- Polygon Streams -> numRings, numVertices
- MultiLineString Streams -> numLineStrings, numVertices
- MultiPolygon Streams -> numPolygons, numRings, numVertices  

A ```Point``` geometry has no entry in the topology column.  
For the streams of the topology column delta and/or rle encoding/bitpacking is used.

#### VertexBuffer
In the ``vertexBuffer`` the vertices of the geometry are stored.
Depending on structure of the geometry one of the following encodings can be used.
- Points
  - Order on a hilbert curve and delta encode with bitpacking -> using the hilbert index instead of the coordinate pairs
- LineString
  - Order the LineStrings with minimal distance to each other and delta encode also for consecutive features with PFOR-Delta
  - Topological Arc Encoding (TAE) for OSM layers like borders
- Polygon 
  - topological encoding for OSM layers like boundary or transportation (in some cases)
  - Shape encoding -> e.g. using the information that lots of buildings are rectangular to use a specific kind of delta encoding or affine transformation


### Feature properties

To support sparse columns a separate ``present`` stream is used for every property which is RLE and Bit-Vector encoded.
Sparse columns are very common especially in OSM datasets for example the name:* property of a feature.

Property data types
- String
  - Plain encoding
    - Parquet quote: "If the dictionary grows too big, whether in size or number of distinct values, the encoding will fall back to the plain encoding"
        -> in the tests with osm data on properties with high entropy the plain fallback doesn't show any real advantages -> really needed?
  - Dictionary encoding
    - Data
      - streams: length and data
    - Index 
      - Bitpacking
      - RLE Encoding and Bitpacking
- Integer (int, uint, sint)
  - Plain
  - Base 128 Varint
  - RLE Bitpacking
  - PFOR-Delta
  - Dictionary Coding?
- Boolean
  - RLE Bit-Vector encoding
- Float/Double
  - Plain
  - Dictionary Coding?

#### Integer compression
In a COVT tile a lot of data are stored in the form of arrays of integers.
For example the coordinates of a geometry or the dictionary indices are stored as integers.
Finding an efficient and fast algorithms for compressing arrays of integers is crucial.


### Vectorization
Using SIMD capable encodings to accelerate the decoding time. 
SIMD instructions are supported in the browser via [WebAssembly](https://v8.dev/features/simd).
The goal is to build only a cross-platform COVTiles decoding library in Rust which can be compiled to WebAssembly
for the usage in a browser.


### Cloud optimized

## File Layout

## Results
Current compression ratios for munich and surroundings with using only some of the above described encodings yet:

![results](assets/results.png)


### Existing formats
Evaluation of building COVT based on the following existing formats:
- ORC
- Parquet
- Protobuf


### References
- https://github.com/mapbox/vector-tile-spec
- https://parquet.apache.org/docs/file-format/data-pages/encodings
- https://orc.apache.org/specification/ORCv1