package org.maplibre.mlt;

import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.decoder.MltDecoder;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;
import org.maplibre.mlt.vector.FeatureTable;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.*;
import org.springmeyer.VectorTileLayer;

/**
 * Benchmarks for the decoding performance of OpenMapTiles schema based tiles into the MVT and MLT
 * in-memory representations.
 */
@State(Scope.Benchmark)
@OutputTimeUnit(TimeUnit.MILLISECONDS)
@BenchmarkMode(Mode.AverageTime)
@Threads(value = 1)
@Warmup(iterations = 5)
@Measurement(iterations = 5)
@Fork(value = 1)
public class OmtDecoderBenchmark {
  /* java-vector-tile library */
  private static final Map<Integer, byte[]> encodedMvtTiles = new HashMap<>();
  /* mapbox-vector-tile-java library */
  private static final Map<Integer, ByteArrayInputStream> encodedMvtTiles2 = new HashMap<>();
  private static final Map<Integer, byte[]> compressedMVTiles = new HashMap<>();
  private static final Map<Integer, byte[]> encodedMltTiles = new HashMap<>();
  private static final Map<Integer, MltTilesetMetadata.TileSetMetadata> tileMetadata =
      new HashMap<>();
  private static final String SEPARATOR = "_";

  @Setup
  public void setup() throws IOException {
    encodeTile(2, 2, 2);
    encodeTile(3, 4, 5);
    encodeTile(4, 8, 10);
    encodeTile(5, 16, 21);
    encodeTile(6, 32, 41);
    encodeTile(7, 66, 84);
    encodeTile(8, 134, 171);
    encodeTile(9, 265, 341);
    encodeTile(10, 532, 682);
    encodeTile(11, 1064, 1367);
    encodeTile(12, 2132, 2734);
    encodeTile(13, 4265, 5467);
    encodeTile(14, 8298, 10748);
  }

  @Setup(Level.Invocation)
  public void resetInputStreams() {
    for (var is : encodedMvtTiles2.values()) {
      is.reset();
    }
  }

  private void encodeTile(int z, int x, int y) throws IOException {
    BenchmarkUtils.encodeTile(
        z,
        x,
        y,
        encodedMvtTiles,
        encodedMvtTiles2,
        compressedMVTiles,
        encodedMltTiles,
        tileMetadata,
        TestSettings.OMT_MVT_PATH,
        SEPARATOR);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ2() {
    var mlTile = encodedMltTiles.get(2);
    var mltMetadata = tileMetadata.get(2);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ3() {
    var mlTile = encodedMltTiles.get(3);
    var mltMetadata = tileMetadata.get(3);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ4() {
    var mlTile = encodedMltTiles.get(4);
    var mltMetadata = tileMetadata.get(4);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ5() {
    var mlTile = encodedMltTiles.get(5);
    var mltMetadata = tileMetadata.get(5);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ6() {
    var mlTile = encodedMltTiles.get(6);
    var mltMetadata = tileMetadata.get(6);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ7() {
    var mlTile = encodedMltTiles.get(7);
    var mltMetadata = tileMetadata.get(7);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ8() {
    var mlTile = encodedMltTiles.get(8);
    var mltMetadata = tileMetadata.get(8);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ9() {
    var mlTile = encodedMltTiles.get(9);
    var mltMetadata = tileMetadata.get(9);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ10() {
    var mlTile = encodedMltTiles.get(10);
    var mltMetadata = tileMetadata.get(10);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ11() {
    var mlTile = encodedMltTiles.get(11);
    var mltMetadata = tileMetadata.get(11);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ12() {
    var mlTile = encodedMltTiles.get(12);
    var mltMetadata = tileMetadata.get(12);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ13() {
    var mlTile = encodedMltTiles.get(13);
    var mltMetadata = tileMetadata.get(13);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ14() {
    var mlTile = encodedMltTiles.get(14);
    var mltMetadata = tileMetadata.get(14);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ2() throws IOException {
    var mvTile = encodedMvtTiles.get(2);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ3() throws IOException {
    var mvTile = encodedMvtTiles.get(3);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ4() throws IOException {
    var mvTile = encodedMvtTiles.get(4);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ5() throws IOException {
    var mvTile = encodedMvtTiles.get(5);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ6() throws IOException {
    var mvTile = encodedMvtTiles.get(6);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ7() throws IOException {
    var mvTile = encodedMvtTiles.get(7);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ8() throws IOException {
    var mvTile = encodedMvtTiles.get(8);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ9() throws IOException {
    var mvTile = encodedMvtTiles.get(9);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ10() throws IOException {
    var mvTile = encodedMvtTiles.get(10);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ11() throws IOException {
    var mvTile = encodedMvtTiles.get(11);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ12() throws IOException {
    var mvTile = encodedMvtTiles.get(12);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ13() throws IOException {
    var mvTile = encodedMvtTiles.get(13);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeMvtMapboxZ14() throws IOException {
    var mvTile = encodedMvtTiles.get(14);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ2() throws IOException {
    var compressedMvTile = compressedMVTiles.get(2);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ3() throws IOException {
    var compressedMvTile = compressedMVTiles.get(3);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ4() throws IOException {
    var compressedMvTile = compressedMVTiles.get(4);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ5() throws IOException {
    var compressedMvTile = compressedMVTiles.get(5);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ6() throws IOException {
    var compressedMvTile = compressedMVTiles.get(6);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ7() throws IOException {
    var compressedMvTile = compressedMVTiles.get(7);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ8() throws IOException {
    var compressedMvTile = compressedMVTiles.get(8);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ9() throws IOException {
    var compressedMvTile = compressedMVTiles.get(9);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ10() throws IOException {
    var compressedMvTile = compressedMVTiles.get(10);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ11() throws IOException {
    var compressedMvTile = compressedMVTiles.get(11);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ12() throws IOException {
    var compressedMvTile = compressedMVTiles.get(12);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ13() throws IOException {
    var compressedMvTile = compressedMVTiles.get(13);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  @Benchmark
  public Map<String, VectorTileLayer> decodeCompressedMvtMapboxZ14() throws IOException {
    var compressedMvTile = compressedMVTiles.get(14);
    var mvTile = EncodingUtils.unzip(compressedMvTile);
    return MvtUtils.decodeMvtMapbox(mvTile);
  }

  /*@Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ2() throws IOException {
    var mvTile = encodedMvtTiles.get(2);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z2() throws IOException {
    var mvTile = encodedMvtTiles2.get(2);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ3() throws IOException {
    var mvTile = encodedMvtTiles.get(3);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z3() throws IOException {
    var mvTile = encodedMvtTiles2.get(3);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ4() throws IOException {
    var mvTile = encodedMvtTiles.get(4);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z4() throws IOException {
    var mvTile = encodedMvtTiles2.get(4);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ5() throws IOException {
    var mvTile = encodedMvtTiles.get(5);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z5() throws IOException {
    var mvTile = encodedMvtTiles2.get(5);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ6() throws IOException {
    var mvTile = encodedMvtTiles.get(6);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z6() throws IOException {
    var mvTile = encodedMvtTiles2.get(6);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ7() throws IOException {
    var mvTile = encodedMvtTiles.get(7);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z7() throws IOException {
    var mvTile = encodedMvtTiles2.get(7);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ8() throws IOException {
    var mvTile = encodedMvtTiles.get(8);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z8() throws IOException {
    var mvTile = encodedMvtTiles2.get(8);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ9() throws IOException {
    var mvTile = encodedMvtTiles.get(9);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z9() throws IOException {
    var mvTile = encodedMvtTiles2.get(9);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ10() throws IOException {
    var mvTile = encodedMvtTiles.get(10);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z10() throws IOException {
    var mvTile = encodedMvtTiles2.get(10);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ11() throws IOException {
    var mvTile = encodedMvtTiles.get(11);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z11() throws IOException {
    var mvTile = encodedMvtTiles2.get(11);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ12() throws IOException {
    var mvTile = encodedMvtTiles.get(12);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z12() throws IOException {
    var mvTile = encodedMvtTiles2.get(12);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ13() throws IOException {
    var mvTile = encodedMvtTiles.get(13);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z13() throws IOException {
    var mvTile = encodedMvtTiles2.get(13);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }

  @Benchmark
  public List<VectorTileDecoder.Feature> decodeMvtZ14() throws IOException {
    var mvTile = encodedMvtTiles.get(14);
    return MvtUtils.decodeMvtFast(mvTile);
  }

  @Benchmark
  public JtsMvt decodeMvt2Z14() throws IOException {
    var mvTile = encodedMvtTiles2.get(14);
    return MvtUtils.decodeMvt2Fast(mvTile);
  }*/

}
