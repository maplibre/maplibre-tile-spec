/**
 * COVT file layout:
 * - Id
 *      - Uint64
 *      - Encodings
 *          - Plain -> SIMD Fast Pfor?
 *          - Delta and RLE Bitpacking Hybird -> when sorted ascending
 * - GeometryType
 *      - Uint8
 *      - Encodings
 *          - Bitpacking (3 bits) and RLE encoding -> RLE Bitpacking Hybrid
 * - Topology
 *      - Point -> no entry
 *      - MultiPoint
 *          - numPoints
 *      - LineString
 *          - numVertices
 *      - MultiLineString
 *          - numLineStrings
 *          - numVertices[]
 *      - Polygon
 *          - numRings
 *          - numVertices[]
 *      - MultiPolygon
 *          - numPolygons
 *          - numRings[]
 *          - numVertices[]
 *     - Encodings
 *          - separate columns
 *          - plain -> SIMD Fast Pfor?
 *          - RLE Bitpacking Hybrid -> also Delta Encoding?
 * - VertexIndex
 *      - Uint32
 *      - Encodings
 *          - plain -> SIMD Fast Pfor?
 *          - Delta Coding -> Patched encoding?
 * - VertexBuffer
 *      - Delta Coding and SIMD Fast Pfor
 *      - Sorted
 *          - Point -> Order on hilbert -> Delta Code and SIMD Fast Pfor
 *          - LineString
 *              - Plain -> Delta code between vertices of LineString and use SIMD Fast Pfor?
 *              - Index Coordinate Encoding -> on a specific redundancy factor use this encoding
 *              - Minimal Distance sorting -> when low redundancy sort LineStrings by minimal distance
 *          - Polygon
 *              - Plain -> Delta code between vertices of LineString and use SIMD Fast Pfor?
 *              - Delta Code between Polygons -> order Endpoints on SFC
 * - Present
 *      - Encodings
 *          - Bit-Vector with RLE-Encoding
 * - Values
 *      - Double
 *      - Boolean
 *          - Bit-Vector with RLE-Encoding
 *      - Integer
 *          - Plain
 *          - RLE Bitpacking
 *          - Dictionary Encoding?
 *      - String
 *          - Encodings
 *              - Plain
 *              - Dictionary
 *                  - Index
 *                      - Bitpacking
 *                      - RLE Bitpacking Hybrid -> RLE Dictionary like Parquet -> for class and subclass
 *                      - Delta encoding, RLE and Bitpacking -> for name property of poi layer with high entropy
 *                          - is this really needed or should for this case plain encoding used
 *                  - Dictionary
 *                      - two streams: length and data
 *                      - length
 *                          - Delta encode length stream? -> or like in ORC RLE v2 encoding
 *                      - data
 *                          - UTF-8 encoding

 **/


