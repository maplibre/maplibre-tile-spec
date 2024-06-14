package com.mlt;

import com.mlt.converter.mvt.MvtUtils;
import com.mlt.decoder.MltDecoder;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import com.mlt.vector.FeatureTable;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.model.JtsMvt;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.TimeUnit;
import no.ecc.vectortile.VectorTileDecoder;
import org.openjdk.jmh.annotations.*;

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
public class BingMapsDecoderBenchmark {
  /* java-vector-tile library */
  private static final Map<Integer, byte[]> encodedMvtTiles = new HashMap<>();
  /* mapbox-vector-tile-java library */
  private static final Map<Integer, ByteArrayInputStream> encodedMvtTiles2 = new HashMap<>();
  private static final Map<Integer, byte[]> encodedMltTiles = new HashMap<>();
  private static final Map<Integer, MltTilesetMetadata.TileSetMetadata> tileMetadata =
      new HashMap<>();
  private static final String SEPARATOR = "-";

  @Setup
  public void setup() throws IOException {
    encodeTile(4, 12, 6);
    encodeTile(5, 16, 11);
    encodeTile(6, 33, 22);
    encodeTile(7, 66, 42);
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
        encodedMltTiles,
        tileMetadata,
        TestSettings.BING_MVT_PATH,
        SEPARATOR);
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
  public FeatureTable[] decodeMltZ4() {
    var mlTile = encodedMltTiles.get(4);
    var mltMetadata = tileMetadata.get(4);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
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
  public FeatureTable[] decodeMltZ5() {
    var mlTile = encodedMltTiles.get(5);
    var mltMetadata = tileMetadata.get(5);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
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
  public FeatureTable[] decodeMltZ6() {
    var mlTile = encodedMltTiles.get(6);
    var mltMetadata = tileMetadata.get(6);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
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
  public FeatureTable[] decodeMltZ7() {
    var mlTile = encodedMltTiles.get(7);
    var mltMetadata = tileMetadata.get(7);
    return MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }
}
