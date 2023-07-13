# Cloud Optimized Vector Tiles (COVTiles)

COVTiles is a case study of how the next generation of vector tiles could look like.  
The main goals of COVTiles are:
- **Improve compression ratio**: minimize the tile size and thereby the network latency (currently up to 5x in some zoom levels compared to MVT)
- **Improve performance**: faster decoding and processing of the tiles (currently up to 3 times faster on selected tiles compared to MVT)
- **Support for fast (cloud optimized) filtering**: Support fast filtering of specific layers and specific property columns like in Parquet
  (predicate and projection pushdown) also in a cloud optimized way via HTTP range requests to reduce the transferred amount of data
- **M values**: Add support for any number of per vertex values. For example, every vertex of a LinesString representing a road can additionally have an elevation (i.e. a Z value in 3D), lane width, curvature, timestamp, and any other extra values.
- **Nested properties of a feature**: based on the Dremel encoding


## Comparison with Mapbox Vector Tiles

### Compression ratio
In the following the sizes of Covt and MVT tiles are compared based on a selected set of tiles.
In the first reduction column, RLE encoding is used for the topology streams.
For the second reduction column patched bitpacking based on SIMD-FastPFOR 128 is used for the topology streams,
which is a way harder to implement and doesn't play that well in combination with a heavyweight compression scheme.

| Zoom level | Tile indices <br/>(minX,maxX,minY,maxY) | Reduction 1 (%)  | Reduction 2(%) |
|------------|-----------------------------------------|------------------|----------------|
| 2          | 2,2,2,2                                 | 36               | 39             | 
| 3          | 4,4,5,5                                 | 29               | 32             |
| 4          |                                         | 71               | 73             | 
| 5          | 16,17,20,21                             | 74               | 75             | 
| 6          | 32,34,41,42                             | 69               | 70             | 
| 7          | 66,68,83,85                             | 68               | 68             | 
| 8          | 132, 135, 170, 171                      | 68               | 68             | 
| 9          | 264, 266, 340, 342                      | 62               |                | 
| 10         | 530, 533, 682, 684                      | 54               |                | 
| 11         | 1062, 1065, 1366, 1368                  | 53               |                | 
| 12         | 2130, 2134, 2733, 2734                  | 54               |                | 
| 13         | 4264, 4267, 5467, 5468                  | 44 (without ICE) |                | 
| 14         | 8296, 8300, 10748, 10749                | 51 (without ICE) |                | 

### Decoding performance
Comparing the decoding performance of a Covt tile with a MVT tile for selected tiles, currently still without
the usage of SIMD instructions:

| Zoom level |  Decoding performance (COVT/MVT)  |      
|------------|:---------------------------------:|          
| 4          |               2.36                | 
| 5          |               2.74                |


## Physical File Layout

The main idea of COVTiles is to use a column oriented layout like the state of the art big data formats Parquet and ORC
or the in-memory format Apache Arrow instead of the row oriented approach on which Mapbox Vector Tiles (MVT) is (implicit) based on.
This enables the usage of custom lightweight compression schemes on a specific column in addition to a tile wide data agnostic compression like Gzip or Brotli.
This can significantly improve the compression ratio. The file layout should be designed SIMD-friendly
to enable the usage of vectorization to significantly improve the decoding performance.

The basic design of a COVT tile is inspired by the MVT layout and is based on the concepts
of a ``layer``and a collection of ``features``.
- Layer -> Name, Features, ...
- Feature -> Id, Geometry, Properties
- Geometry -> Based on the Simple Feature Access Model (SFA) of the OGC

A layer can consist of the following columns.

### Id Column
| Datatype |        Encodings         |      
|----------|:------------------------:|          
| Uint64   | Plain, Delta Varint, RLE |


### Geometry Column

The information about the geometry of the features is separated in different streams and is inspired
by the [geoarrow](https://github.com/geoarrow/geoarrow) format. Using separate streams for describing the geometry of a feature enables a
better optimization of the compression and faster processing. In addition, pre-tessellated meshes of polygons
can be stored directly in the file to avoid the time-consuming triangulation step.  
A geometry column can consist of the following streams:

| Stream name     | Data type |         encoding          |       
|-----------------|:---------:|:-------------------------:|           
| GeometryType    |   Byte    |         Byte RLE          |
| GeometryOffsets |  UInt32   |            RLE            |
| PartOffsets     |  UInt32   |            RLE            |
| RingsOffsets    |  UInt32   |            RLE            |
| VertexOffsets   |  UInt32   |       Delta Varint        |
| IndexBuffer     |  UInt32   | Delta Patched Bitpacking? |
| VertexBuffer    |   Int32   |       Delta Varint        |

#### GeometryType Stream

The geometryType stream encodes one of the ``OGC Simple Feature Access Model`` specified geometry types.
A layer can have different geometry types, but as most layer contain only geometries of a single type.
By using a separate stream, the geometry can be efficiently compressed with an RLE encoding inspired by
the [ORC](https://orc.apache.org/specification/ORCv1/) file format.
The GeometryType stream could be omitted when a fixed layout is used but is currently maintained
to enable a variable number of streams and to simply the processing.

#### Topology Streams
Different topology streams are used to describe the structure of a geometry depending on the type of the geometry.  
The layout of the topology streams is inspired by the [GeoArrow](https://github.com/geoarrow/geoarrow/blob/main/format.md) format.
In the topology streams only the offsets to the vertices are stored.   
This enables fast/random access to specific geometries of a layer and can improve the decoding/processing performance.  
For reducing the size of the topology column, RLE encoding inspired by
the [ORC](https://orc.apache.org/specification/ORCv1/) file format is used.
A combination of delta encoding and the [RLE / Bit-Packing Hybrid](https://parquet.apache.org/docs/file-format/data-pages/encodings/#a-namerlearun-length-encoding--bit-packing-hybrid-rle--3)
encoding of the Parquet format has shown even better results in the evaluation but would increase complexity by introducing an
additional RLE encoding.

#### VertexOffsets Stream (optional)
Used in combination with the [Indexed Coordinate Encoding (ICE)](https://towardsdatascience.com/2022-051-better-compression-of-gis-features-9f38a540bda5#a8bf).
Stores the index to a vertex in the Vertex Buffer.
This can minimize the size of the VertexBuffer when LineString or Polygon geometries share the same vertices.
In the OpenMapTiles scheme this is for example the case for the ``transportation`` layer, where in some zoom levels the number of vertices
can be reduced by factor 5.

#### IndexBuffer Stream (optional)
The ``IndexBuffer`` stream stores the indices of the triangulated polygons.
Performing benchmarks on an OSM dataset showed that even with the fast polygon triangulation library [earcut](https://github.com/mapbox/earcut),
polygon tessellation can take up to 100 milliseconds or more at some zoom levels.
In combination with the `VertexBuffer` this can allow zero-copy of polygon geometries with direct upload to GPUs,
inspired by the glTF format used in the 3DTiles specification.
The best encoding for the compression has yet to be evaluated, but is likely a combination of delta encoding
with a patched binary packing like FastPfor128.

#### VertexBuffer Stream
In the ``VertexBuffer`` the vertices of the geometry are stored.
By having the vertices in a continuous vertex buffer enables fast processing and in some cases zero copy to format which can be consumed by the GPU.
When the plain encoding is ued for the VertexBuffer, the vertex coordinates are delta and Varint encoded per geometry.
When ICE encoding is used the vertices are sorted on a ``Hilbert curve`` and all vertices are delta and Varint encoded.
A patched bitpacking method like FastPfor128 has shown even better results in the evaluation but would increase complexity by introducing an
additional encoding.

Depending on the type of geometry the geometry column can have the following streams:
- Point: VertexBuffer
- LineString: Part offsets, Vertex offsets, VertexBuffer
- Polygon: Part offsets (Polygon), Ring offsets (LinearRing), Vertex offsets, IndexBuffer, VertexBuffer
- MultiPoint: Geometry offsets, VertexBuffer
- MultiLineString: Geometry offsets, Part offsets (LineString), Vertex offsets, VertexBuffer
- MultiPolygon: Geometry offsets, Part offsets (Polygon), Ring offsets (LinearRing), Vertex offsets, IndexBuffer, VertexBuffer

### Property Columns

The layout for the properties of a Feature is inspired by the ORC format, which uses different streams for encoding the properties in an efficient way.
Depending on the data type of the value of the feature property different streams are used.
For all data types a `Present` stream is used for effectively compressing sparse property columns.
Sparse columns are very common especially in OSM datasets for example the name:* property of a feature.

| Data type |                    Encoding                    |              Streams              |       
|-----------|:----------------------------------------------:|:---------------------------------:|           
| Boolean   |              Bitset and Byte RLE               |           Present, Data           |
| Uint64    |                 Varint or RLE                  |           Present, Data           |
| Int64     |                 Varint or RLE                  |           Present, Data           |
| Float     |                                                |           Present, Data           |
| Double    |                                                |           Present, Data           |
| String    | Plain, Dictionary or <br/>Localized Dictionary | Present, Data, Length, Dictionary |

The localized dictionary encoding of string columns shares a common dictionary over different localized columns.
For example the values of the name:* columns of an OSM dataset can be identical across columns.
Therefore, this encoding enables the efficient compression of localized values.

## Logical File Layout
The internal layout after decoding is inspired by the Apache Arrow memory format.  
In the future COVTiles could be directly converted in the Arrow format, to allow
fast processing of the tile.

## Encodings

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

Depending on the structure of the geometry the following evaluated approaches improves the overall tile size:
- Points
    - Order on a hilbert curve and delta encode with patched bitpacking
    - Using the hilbert index instead of the coordinate pairs
- LineString
    - Order the LineStrings with minimal distance to each other and delta encode also for consecutive features with PFOR-Delta encoding
    - [Indexed Coordinate Encoding (ICE)](https://towardsdatascience.com/2022-051-better-compression-of-gis-features-9f38a540bda5#a8bf) e.g. for OSM layers like transportation
    - [Topological Arc Encoding (TAE)](https://towardsdatascience.com/2022-051-better-compression-of-gis-features-9f38a540bda5#8620)
- Polygon
    - Indexed Coordinate Encoding (ICE)
    - Topological Arc Encoding (TAE)
    - Shape encoding -> e.g. using the information that lots of buildings are rectangular to use a specific kind of delta encoding or affine transformation

Because of the additional complexity and relatively smaller savings in size not all encodings are used in the current version.

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


## Filtering

COVTiles should enable fast filtering of specific layers and specific property columns like in Parquet
(predicate and projection pushdown).

### Cloud optimized filtering (Predicate Pushdown)

Only specific layers of a tile based on the used style can be requested.
Depending on the style only a subset of a tile may be needed, for example a light theme like the OpenMapTiles `Positron` style
does not render the POI layer, the `Basic` style on the other hand does.
This has the advantage that only one tileset for different styles is needed and still only the required data can be requested.
The basic idea is to make a vector tile "cloud optimized", so that specific combinations of layers can be fetched via HTTP range requests
from a tileset hosted on a cloud object storage or an CDN. CDNs like AWS CloudFront also support multipart ranges,
so the latency and costs shouldn't be that much higher for fetching a tile, nevertheless an overhead for an additional index (maybe as a sidecar)
on a cloud optimized format like COMTiles would be created.

### Existing formats
Evaluation of building COVT based on the following existing formats:
- ORC
- Parquet -> first tests showed a significantly worse compression ratio
- Protobuf



### References
- https://github.com/mapbox/vector-tile-spec
- https://parquet.apache.org/docs/file-format/data-pages/encodings
- https://orc.apache.org/specification/ORCv1
