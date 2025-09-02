package com.mlt;

import com.mlt.converter.MltConverter;
import com.mlt.converter.mvt.MvtUtils;
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
import org.springmeyer.VectorTileLayer;

/**
 * Benchmarks for the decoding performance of Bing Maps based tiles into the MVT and MLT in-memory
 * representations.
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
  private static final Map<Integer, byte[]> compressedMVTiles = new HashMap<>();
  private static final Map<Integer, byte[]> encodedMltTiles = new HashMap<>();
  private static final Map<Integer, byte[]> tileMetadata = new HashMap<>();
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
        compressedMVTiles,
        encodedMltTiles,
        tileMetadata,
        TestSettings.BING_MVT_PATH,
        SEPARATOR);
  }

  private FeatureTable[] decodeVectorized(int n) throws IOException {
    var mlTile = encodedMltTiles.get(n);
    var mltMetadata =
        MltConverter.parseEmbeddedMetadata(new ByteArrayInputStream(tileMetadata.get(n)));
    // Vectorized decoding currently disabled
    return new FeatureTable[0]; // MltDecoder.decodeMlTileVectorized(mlTile, mltMetadata);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ4() throws IOException {
    return decodeVectorized(4);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ5() throws IOException {
    return decodeVectorized(5);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ6() throws IOException {
    return decodeVectorized(6);
  }

  @Benchmark
  public FeatureTable[] decodeMltZ7() throws IOException {
    return decodeVectorized(7);
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
}
