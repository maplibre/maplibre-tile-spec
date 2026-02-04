package org.maplibre.mlt;

import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.GeometryFactory;
import org.maplibre.mlt.cli.CliUtil;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.decoder.MltDecoder;

public class MltFixtureGenerator {

  private static final int TILE_EXTENT = 4096;
  private static final long RANDOM_SEED = 1;
  private static final String OUTPUT_DIR = "../../test/synthetic";

  // Object generation
  private static Feature generateSinglePoint(GeometryFactory gf, long id) {
    var rand = new java.util.Random(RANDOM_SEED);
    int x = rand.nextInt(TILE_EXTENT);
    int y = rand.nextInt(TILE_EXTENT);
    var geom = gf.createPoint(new Coordinate(x, y));
    return new Feature(id, geom, Map.of("name", "Point " + id));
  }

  private static Feature generateSimplePolygon(
      GeometryFactory gf, long id, double x, double y, double size) {
    var geom =
        gf.createPolygon(
            new Coordinate[] {
              new Coordinate(x, y),
              new Coordinate(x + size, y),
              new Coordinate(x + size, y + size),
              new Coordinate(x, y + size),
              new Coordinate(x, y)
            });
    int area = (int) geom.getArea();
    return new Feature(id, geom, Map.of("name", "Simple Polygon " + id, "area", area));
  }

  // Layer generation
  private static Layer generatePointsLayer(GeometryFactory gf, long[] nextId) {
    var f1 = generateSinglePoint(gf, nextId[0]++);
    var f2 = generateSinglePoint(gf, nextId[0]++);
    return new Layer("points", List.of(f1, f2), TILE_EXTENT);
  }

  private static Layer generatePolygonsLayer(GeometryFactory gf, long[] nextId) {
    var f1 = generateSimplePolygon(gf, nextId[0]++, 500, 500, 1000);
    var f2 = generateSimplePolygon(gf, nextId[0]++, 1000, 1000, 1000);
    return new Layer("polygons", List.of(f1, f2), TILE_EXTENT);
  }

  // MapboxVectorTile tile generation
  // Note: MapboxVectorTile is in-memory object structure (it is not MVT file)
  private static MapboxVectorTile buildPointsTile(GeometryFactory gf) {
    long[] nextId = {1};
    var layer = generatePointsLayer(gf, nextId);
    return new MapboxVectorTile(List.of(layer));
  }

  private static MapboxVectorTile buildPolygonsTile(GeometryFactory gf) {
    long[] nextId = {1};
    var layer = generatePolygonsLayer(gf, nextId);
    return new MapboxVectorTile(List.of(layer));
  }

  private static MapboxVectorTile buildMixedGeometriesTile(GeometryFactory gf) {
    long[] nextId = {1};
    var pointsLayer = generatePointsLayer(gf, nextId);
    var polygonsLayer = generatePolygonsLayer(gf, nextId);
    return new MapboxVectorTile(List.of(pointsLayer, polygonsLayer));
  }

  private static Map<String, MapboxVectorTile> generateTileGeometry() {
    var gf = new GeometryFactory();
    var tileGeometry = new HashMap<String, MapboxVectorTile>();
    tileGeometry.put("points", buildPointsTile(gf));
    tileGeometry.put("polygons", buildPolygonsTile(gf));
    tileGeometry.put("mixed", buildMixedGeometriesTile(gf));
    return tileGeometry;
  }

  // Simple fixture case: just the output name, tile to use, and config
  private record FixtureCase(String outputName, String tileGeometryKey, ConversionConfig config) {}

  // Define all the fixture cases - just pick short names that make sense
  private static List<FixtureCase> defineFixtureCases() {
    var outlineNames = List.<String>of();
    var plain = ConversionConfig.IntegerEncodingOption.PLAIN;
    var delta = ConversionConfig.IntegerEncodingOption.DELTA;
    var rle = ConversionConfig.IntegerEncodingOption.RLE;

    return List.of(
        new FixtureCase(
            "points-plain",
            "points",
            ConversionConfig.builder()
                .includeIds(true)
                .useFastPFOR(false)
                .useFSST(false)
                .coercePropertyValues(false)
                .useMortonEncoding(false)
                .preTessellatePolygons(false)
                .outlineFeatureTableNames(outlineNames)
                .integerEncoding(plain)
                .build()),
        new FixtureCase(
            "polygons-fsst-delta",
            "polygons",
            ConversionConfig.builder()
                .includeIds(true)
                .useFastPFOR(false)
                .useFSST(true)
                .coercePropertyValues(false)
                .useMortonEncoding(false)
                .preTessellatePolygons(false)
                .outlineFeatureTableNames(outlineNames)
                .integerEncoding(delta)
                .build()),
        new FixtureCase(
            "mixed-fastpfor-rle",
            "mixed",
            ConversionConfig.builder()
                .includeIds(true)
                .useFastPFOR(true)
                .useFSST(false)
                .coercePropertyValues(false)
                .useMortonEncoding(false)
                .preTessellatePolygons(false)
                .outlineFeatureTableNames(outlineNames)
                .integerEncoding(rle)
                .build()));
  }


  // Export the MLT fixture to the output directory, plus a JSON dump of the decoded tile for readability
  private static void exportMltFixture(
      String outputName, MapboxVectorTile tileGeometry, ConversionConfig config)
      throws IOException {
    var metadata =
        MltConverter.createTilesetMetadata(tileGeometry, Map.of(), config.getIncludeIds());
    byte[] mltData = MltConverter.convertMvt(tileGeometry, metadata, config, null);
    var outputPath = Paths.get(OUTPUT_DIR, outputName + ".mlt");
    Files.createDirectories(outputPath.getParent());
    Files.write(outputPath, mltData);
    var decodedTile = MltDecoder.decodeMlTile(mltData);
    String jsonOutput = CliUtil.printMLT(decodedTile);
    var jsonOutputPath = Paths.get(OUTPUT_DIR, outputName + ".json");
    Files.write(jsonOutputPath, jsonOutput.getBytes(StandardCharsets.UTF_8));
  }

  private static void generateMltFixtures() throws IOException {
    Files.createDirectories(Paths.get(OUTPUT_DIR));
    var tileGeometry = generateTileGeometry();
    for (var c : defineFixtureCases()) {
      exportMltFixture(c.outputName(), tileGeometry.get(c.tileGeometryKey()), c.config());
    }
  }

  public static void main(String[] args) throws IOException {
    generateMltFixtures();
  }

  @Test
  @Disabled("Only for generating fixtures")
  void testGenerateMltFixtures() throws Exception {
    generateMltFixtures();

    for (var c : defineFixtureCases()) {
      var name = c.outputName();
      assertTrue(Files.isRegularFile(Paths.get(OUTPUT_DIR, name + ".mlt")), name + ".mlt");
      assertTrue(Files.isRegularFile(Paths.get(OUTPUT_DIR, name + ".json")), name + ".json");
    }
  }
}
