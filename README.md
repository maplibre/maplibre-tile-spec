# Cloud Optimized Vector Tiles (COVTiles)

The main idea of COVTiles is to use a column oriented layout like the state of the art big data formats Parquet and ORC
or the in-memory format Apache Arrow instead of the row oriented approach on which Mapbox Vector Tiles (MVT) is based on.
This enables the usage of custom lightweight compression schemes on a specific column in addition to a tile wide data agnostic compression like Gzip or Brotli.
This can significantly improve the compression ratio.
For improving the decoding performance SIMD instructions are used.
In addition the tiles are "cloud optimized" which means that also only specific layers of a tile can be requested 
from a cloud storage via HTTP range requests.

The main goals of COVTiles are:
- Minimize the tile size and thereby the network latency (currently up to 5x in some zoom levels compared to MVT)
- Faster decoding through vectorization (SIMD)
- Using the concepts of the cloud optimized file formats to enable the filtering of only specific layers of a tile via HTTP range requests from a cloud storage (Predicate Pushdown)

## Mapbox Vector Tiles (MVT)
[Mapbox Vector Tiles (MVT)](https://github.com/mapbox/vector-tile-spec) are the state of the art way
of storing and visualizing planet scale spatial vector tilesets.
They are optimized in size through the usage of a local tile coordinate system, delta and Base128 Varint encoding
for the geometries as well as dictionary encoding for the keys and values of the feature properties.


## File Layout
The COVTiles format is based on a colum oriented layout with custom lightweight compression schemes per column.

The basic design of a COVT tile is inspired by the MVT layout and is based on the concepts
of a ``layer``and a collection of ``features``.
- Layer -> Name, Features, ...
- Feature -> Id, Geometry, Properties
- Geometry -> Based on the Simple Feature Access Model (SFA) of the OGC

### Id Column

### Geometry Columns

The information about the geometry of a feature is separated in a ``geometryType``, ``topology``, ``indexBuffer`` and ``vertexBuffer`` column.
Using separate columns for describing the geometry of a feature enables a better optimization of the compression.

#### GeometryType Column

The geometryType column encodes one of the ``OGC Simple Feature Access Model`` specified geometry types.
A layer can have different geometry types, but as most layer contain only geometries of a single type.
By using a separate column, the geometry can be efficiently compressed with [RLE / Bit-Packing Hybrid](https://parquet.apache.org/docs/file-format/data-pages/encodings/#a-namerlearun-length-encoding--bit-packing-hybrid-rle--3) encoding
of the Parquet format -> This column could be completely omitted when in the topology column a fixed layout is used

#### Topology Column
The ``topology`` column uses different streams (inspired by the ORC file format) depending on the used geometry type for describing the structure of a geometry.  
The layout of the topology column is inspired by the [GeoArrow](https://github.com/geoarrow/geoarrow/blob/main/format.md) format.
In the topology column only the offsets to the vertices are stored.   
This for example has the advantage when COVT is used in combination with the Mapbox Style specification, where features are often filtered from a source-layer based on the properties.
Enabling fast/random access to specific features of a layer can improve the decoding performance.   
For reducing the size of the topology column a combination of delta encoding and the [RLE / Bit-Packing Hybrid](https://parquet.apache.org/docs/file-format/data-pages/encodings/#a-namerlearun-length-encoding--bit-packing-hybrid-rle--3) encoding of the Parquet format is used.

Depending on the geometry type the topology column has the following streams:
- Point: no stream
- LineString: Part offsets
- Polygon: Part offsets (Polygon), Ring offsets (LinearRing)
- MultiPoint: Geometry offsets
- MultiLineString: Geometry offsets, Part offsets (LineString)
- MultiPolygon: Geometry offsets, Part offsets (Polygon), Ring offsets (LinearRing)

#### IndexBuffer Column (optional)
Used in combination with the [Indexed Coordinate Encoding (ICE)](https://towardsdatascience.com/2022-051-better-compression-of-gis-features-9f38a540bda5#a8bf).
Stores the index to a coordinate in the Vertex Buffer.
This can minimize the size of the VertexBuffer when LineString or Polygon geometries share the same vertices.
In the OpenMapTiles scheme this is for example the case for the ``transportation`` layer, where in some zoom levels the number of vertices
can be reduced by factor 5.
The vertices are sorted on a ``Hilbert curve`` and delta encoded with a patched encoding.
For the IndexBuffer delta encoding and bitpacking is used.

#### VertexBuffer Column
In the ``vertexBuffer`` the vertices of the geometry are stored.
Depending on structure of the geometry one of the following encodings can be used.
- Points
  - Order on a hilbert curve and delta encode with bitpacking -> using the hilbert index instead of the coordinate pairs
- LineString
  - Order the LineStrings with minimal distance to each other and delta encode also for consecutive features with PFOR-Delta encoding
  - Indexed Coordinate Encoding (ICE) e.g. for OSM layers like transportation
  - Topological Arc Encoding (TAE)
- Polygon
    - Indexed Coordinate Encoding (ICE)
    - Topological Arc Encoding (TAE)
    - Shape encoding -> e.g. using the information that lots of buildings are rectangular to use a specific kind of delta encoding or affine transformation

### Feature Properties Column

Inspired by the ORC format which uses different streams for encoding the properties in an efficient way.
Depending on the data type of the value of the feature property different streams are used.
For all data types a `Present` stream is used for effectively compressing sparse property columns.
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


## Compression

In a COVT tile a lot of data are stored in the form of arrays of integers.
For example the coordinates of a geometry or the dictionary indices are stored as integers.
Finding an efficient and fast algorithms for compressing arrays of integers is crucial.
To achieve a good compression ratio a logical and physical level compression technique for null suppression are used
in most columns.    

### Logical level
The following logical level techniques can be used for the compression of the different columns:
- Run-length encoding (RLE)
- Delta encoding
- Dictionary encoding

### Physical level
The following null suppression techniques have been evaluated:
- Byte aligned
  - Base 128 Varint
- Bit aligned
  - SIMD-BP128
  - FastPfor128 
  - NetPFD
  - OptPFD

In the evaluation the bit aligned null suppression algorithms for the integer compression showed a better compression ratio for the column of a COVT tile 
compared to the byte aligned encodings.

### Combination

The ORC and Parquet file format already offer the following effective encodings which combine the logical and physical level:
- Parquet [RLE / Bit-Packing Hybrid](https://parquet.apache.org/docs/file-format/data-pages/encodings/#a-namerlearun-length-encoding--bit-packing-hybrid-rle--3)  
  Uses a combination of bit-packing and run length encoding to more efficiently store repeated value.
  Shows good results in combination with delta encoding for sored ``Id`` layers.
- Parquet [Delta Encoding](https://parquet.apache.org/docs/file-format/data-pages/encodings/#a-namedeltaencadelta-encoding-delta_binary_packed--5)
- ORC Integer Run Length Encoding version 1
- ORC Integer Run Length Encoding version 2

### Vectorized Combination
Vectorized RLE, Delta and Dictionary coding -> https://wwwdb.inf.tu-dresden.de/wp-content/uploads/T_2014_Master_Patrick_Damme.pdf

### Id compression
In the following different integer compression algorithms for the `Id` column of an OSM dataset has been evaluated:  
![id compression](assets/Osm_zoom5_transportation_compression.png)

### Geometry compression
The following approaches have been evaluated:
- Delta encode between vertices of a geometry
- Delta encode between geometries
  - SFC ordering -> Point and Polygon
  - Minimal Distance Ordering -> LineString
- [Indexed Coordinate Encoding (ICE)](https://towardsdatascience.com/2022-051-better-compression-of-gis-features-9f38a540bda5#a8bf)
- [Topological Arc Encoding (TAE)](https://towardsdatascience.com/2022-051-better-compression-of-gis-features-9f38a540bda5#8620)
- Shape coding -> building
- Using hilbert index instead of coordinate pairs


## Tile processing performance 

In the following important aspects for fast data decoding are listed.
These topics are also main considerations in the different in-memory and serialization formats like Apache Arrow, Cap'n Proto and FlatBuffers.
- Vectorization (SIMD) -> currently only in the browser via WebAssembly which introduces some overhead because of an additional copy step
- Cache Locality -> column oriented
- Data Alignment -> tradeoff between size and performance
- Pipelining
- Zero-Copy   
  -> hard to achieve in the modern cartography compared to real 3D geospatial data formats like GLTF/3DTiles    
-> Use Apache Arrow as in memory representation for the tile processing

### Vectorization
Using SIMD capable encodings to accelerate the decoding time.
SIMD instructions are supported in the browser via [WebAssembly](https://v8.dev/features/simd).
The goal is to build only a cross-platform COVTiles decoding library in Rust which can be compiled to WebAssembly
for the usage in a browser.


## Cloud optimized filtering -> Predicate Pushdown

Only specific layers of a tile based on the used style can be requested.
Depending on the style only a subset of a tile may be needed, for example a light theme like the OpenMapTiles `Positron` style
does not render the POI layer, the `Basic` style on the other hand does.
This has the advantage that only one tileset for different styles is needed and still only the required data can be requested.
The basic idea is to make a vector tile "cloud optimized", so that specific combinations of layers can be fetched via HTTP range requests
from a tileset hosted on an cloud object storage or an CDN. CDNs like AWS CloudFront also support multipart ranges,
so the latency and costs shouldn't be that much higher for fetching a tile, nevertheless a overhead for an additional index (maybe as an sidecar)
on a cloud optimized format like COMTiles would be created.  


## First Results
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