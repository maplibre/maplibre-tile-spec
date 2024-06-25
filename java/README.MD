
> **_Notice:_** This experimental Java encoder uses the [FSST](https://www.vldb.org/pvldb/vol13/p2649-boncz.pdf) encoding for efficiently encoding
substrings of a tile. In the current research state only a quick solution has been implemented, with the restrication that only a windows build
> of the [FSST project](https://github.com/cwida/fsst) is available.

Generate Java metadata classes from ``maplibre_tile_metdata.proto`` file:
```console
protoc --java_out=src/main/java -I "..\..\spec\schema" mlt_tileset_metadata.proto
```

To use the polygon pre-tessellation feature currently a separate Node.js application has to be started in the 
tessellation directory via
```console
npm run serve
```

