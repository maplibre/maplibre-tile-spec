---
description: Choosing encodings and decoding into memory
---

<h1>Implementation Guide</h1>

---

This page is guidance for building an encoder or a decoder.
The wire format is defined by the [v1](specification/v1.md) and [v2](specification/v2.md) specifications, not here.

## Choosing an Encoding

A brute-force search over every combination is too costly.
Use the selection strategy from the [BTRBlocks](https://www.cs.cit.tum.de/fileadmin/w00cfj/dis/papers/btrblocks.pdf) paper:

- Calculate data metrics to exclude unsuitable encodings early (e.g., exclude RLE if the average run length is less than 2).
- Use a sampling-based algorithm: randomly select parts of the data totaling ~1% of the full dataset and apply the candidate encodings from step 1.
  Choose the scheme that produces the smallest output.

Compare stored bytes, not an estimate.
Do not assume a later gzip pass changes which candidate wins.

## In-Memory Format

The record-oriented, array-of-structures in-memory model used by libraries processing Mapbox Vector Tiles incurs considerable overhead.
This includes creating many small objects (increasing memory allocation load) and placing additional strain on garbage collectors in browsers.

MLT uses a columnar memory layout (data-oriented design) for its in-memory format to overcome these issues.
This approach improves cache utilization for subsequent data access and enables the use of fast SIMD instructions.
The MLT in-memory format incorporates ideas from analytical in-memory formats like Apache Arrow, Velox, and the DuckDB execution format, tailored for visualization use cases.
It is also designed for future parallel processing on the GPU within compute shaders.

The main design goals are:

- Define a platform-agnostic representation to avoid expensive materialization costs, especially for strings.
- Maximize CPU throughput by optimizing memory layout for cache locality and SIMD instructions.
- Allow random (preferably constant-time) access to all data for parallel processing on GPUs (compute shaders).
- Provide compressed data structures that can be processed directly without full decoding.
- Provide tile geometries in a representation that can be loaded into GPU buffers with minimal additional processing.

Data is stored in contiguous memory buffers called **vectors**, accompanied by metadata and an optional null bitmap.
An auxiliary offset buffer enables random access to variable-sized data types like strings or lists.
The supported vector types are:

- [Flat Vectors](https://duckdb.org/internals/vector.html#flat-vectors)
- [Constant Vectors](https://duckdb.org/internals/vector.html#constant-vectors)
- [Sequence Vectors](https://duckdb.org/internals/vector.html#sequence-vectors)
- [Dictionary Vectors](https://duckdb.org/internals/vector.html#dictionary-vectors)
- FSST Dictionary Vectors
- Shared Dictionary Vectors
- [Run-End Encoded (REE) Vectors](https://arrow.apache.org/docs/format/Columnar.html#run-end-encoded-layout)

Using a compressed vector where possible makes the conversion from storage to in-memory format essentially a zero-copy operation.

Following Apache Arrow's approach and the [Intel performance guide](https://www.intel.com/content/www/us/en/developer/topic-technology/data-center/overview.html), decoders should allocate memory on addresses aligned to a 64-byte multiple (where possible).

!!! NOTE
    Further evaluation is needed to determine if [recent research](https://arxiv.org/pdf/2306.15374.pdf) can enable random access on delta-encoded values.
