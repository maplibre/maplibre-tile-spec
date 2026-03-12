## MLT Encoder Java CLI Application

This application currently only supports converting MVT tiles to MLT format.

It can convert standalone tile files, MBTiles files, MLN Offline database files, and PMTiles files.

### Usage

See the `--help` output for the full list of options.

At a minimum, the input file(s) must be specified with `--mvt`, `--mbtiles`, `--offlinedb`, or `--pmtiles`.

In the case of `--pmtiles`, the input may be a URL, in which case the data will be seletively downloaded, allowing rapid conversion of zoom-level subsets.

By default, the output file will be the input basename with the `.mlt` extension added, placed in the working directory.  This can be overridden with the `--mlt` option.  The `--dir` option changes the output directory.

For efficient conversion the `--parallel` option should generally be used to take advantage of all available CPU cores.

### Environment Variables

#### Common

- `MLT_TILE_LOG_INTERVAL`: Controls the number of tiles between progress reports with `--verbose`

#### MBTiles

Tile compression within an MBTiles file is optional.  The encoder will only store compressed MLT tiles if they are slightly smaller than the uncompressed MLT data.  To be stored compressed, a tile must meet both criteria, as controlled by the following environment variables:

- `MLT_COMPRESSION_RATIO_THRESHOLD`: Minimum compression ratio to apply compression (default: 0.98)
  - If compressed tile is larger than his fraction of the uncompressed tile, it will be discarded and the uncompressed tile will be stored instead.
  - This value may be greater than 1.0 to indicate that tiles should be stored in compressed form even if it increases their size.
- `MLT_COMPRESSION_FIXED_THRESHOLD`: Minimum savings in bytes for a tile to be compressed (default:  20)
  - If the compressed tile size plus this value is greater than the uncompressed tile size, the compressed tile will be discarded and the uncompressed tile will be stored instead.
  - This value can be negative to indicate that tiles should be stored in compressed form even if they are larger than the uncompressed tile.

#### PMTiles

PMTiles files can be extremely large, and may benefit from careful selection of extended configuration parameters.

PMTiles conversion requires about X bytes of memory per tile.

- `MLT_CACHE_MAX`: Maximum cache size in bytes.
- `MLT_CACHE_MAX_HEAP_PERCENT`: Maximum cache size as a percentage of maximum heap size.
  - If a percentage is specified, it will take precedence over `MLT_CACHE_MAX`.
- `MLT_CACHE_EXPIRE`: Cache expiration duration after access, in ISO-8601 (e.g. P1.2S) or plain (e.g., 1.2s).
  - This is generally not important, but may decrease memory use with a very large cache in long-running conversions.
- `MLT_CACHE_BLOCK_SIZE`: Aligned block size of cached data, in bytes
  - Cache misses will trigger a read of the entire block(s) containing the tile, often allowing reads of nearby tiles without an additional request.
  - When zero, only the requested tile will be read on a cache miss.  This is not recommended.
  - Larger block sizes mean fewer entries in the cache, but tend to be more efficient as the main benefit is from other tiles in the same block.
- `MLT_CACHE_AVERAGE_SIZE`: Average size of tiles in bytes
  - This is used to estimate the number of tiles that can be stored in the cache.
  - A value of zero disables cache pre-allocation.
- `MLT_MAX_TILE_TRACK_SIZE`: Maximum size of a tile to be tracked in memory, in bytes.
  - PMTiles files combine ranges of identical tiles (in Hilbert index order) into a single directory entry. If the same tile contents appears elsewhere, however, it will be stored as a separate entry. The encoder keeps track of each input tile in order to avoid re-converting it if it's encountered later, reducing the computation needed at the cost of some memory.
  - Only small tiles, e.g., representing empty ocean, are commonly duplicated and so worth tracking.  So to save memory, only tiles below this threshold are tracked.
  - Set to zero to disable tracking altogether.
- `MLT_THREAD_QUEUE_SIZE`: The maximum number of items (per worker thread) to queue when parallel processing is enabled.
  - Larger values cause the directory traversal to finish earlier allowing for a progress percentage to be displayed, at the cost of more memory use.
  - Very small values may lead to queue starvation and reduced performance.

In addition, it may be beneficial to adjust the JVM heap parameters to allow for a heap size larger than the default, e.g.:
  - `-Xmx32G` to allow up to 64 GB of heap memory
  - `-Xms16G` to pre-allocate 16 GB of heap memory at startup

Cache hits are more important when using a remote source, but a very large cache (1GB+) seems to be counterproductive.

Use `--cache-stats` to print cache statistics to help tune the cache configuration parameters.
