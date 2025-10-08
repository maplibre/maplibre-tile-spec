# Mlt Decoder rs

# Run-Length Encoding (RLE) encoder/decoder
Despite the fact that there are other crates that can encode/decode RLE, a custom RLE decoder has been implemented in Rust to support the logic of the MLT decoder.

# Regenerating binary test files
The java code can be used to break the output MLT files into tiny binary snippets for testing purposes. The java code is located in the project root `java` dir. While there, run the following commands. You must have [`fd`](https://github.com/sharkdp/fd) installed.

```bash
cd ../../java
# build the jar
./gradlew cli
# run the jar to generate the binary snippets
fd . ../test/fixtures -e pbf -x java -jar build/libs/encode.jar --mvt {} --mlt $(tmp='{.}'; echo ${tmp/\/fixtures\///expected/}).mvt --rawstreams --tessellate --outlines ALL --verbose
```

The resulting files are placed in the `test/fixtures/expected` dir of this project.
