
> **_Notice:_** This experimental Java encoder uses the [FSST](https://www.vldb.org/pvldb/vol13/p2649-boncz.pdf) encoding for efficiently encoding
substrings of a tile. In the current research state only a quick solution has been implemented, with the restrication that only a windows build of the [FSST project](https://github.com/cwida/fsst) is available.

To build command line tools:
```console
cd java
./gradlew cli
```

Example of converting MVT to MLT:

```console
java -jar build/libs/encode.jar --mvt ../test/fixtures/simple/point-boolean.pbf --mlt /tmp/point-boolean.mlt --tessellate --outlines ALL  --embedmetadata --verbose

java -jar build/libs/encode.jar --mbtiles /Users/timsylvester/src/ml/ml-tile-spec/test/omt-ref.mbtiles --mlt /tmp/omt-basic-mlt.mbtiles --tessellate --outlines ALL --compress=deflate --verbose
```

To use the remote polygon pre-tessellation feature (using `--tessellateurl http://localhost:3000`) currently a separate Node.js application has to be started in the tessellation directory via:
```console
npm run serve
```
