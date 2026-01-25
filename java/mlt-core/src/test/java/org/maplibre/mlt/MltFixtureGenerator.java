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

  private static Map<String, MapboxVectorTile> generateTiles() {
    var gf = new GeometryFactory();
    var tiles = new HashMap<String, MapboxVectorTile>();
    tiles.put("points", buildPointsTile(gf));
    tiles.put("polygons", buildPolygonsTile(gf));
    tiles.put("mixed", buildMixedGeometriesTile(gf));
    return tiles;
  }

  // Config generation
  private static Map<String, ConversionConfig> generateConversionConfigs() {
    var configs = new HashMap<String, ConversionConfig>();
    var outlineNames = List.<String>of();
    var auto = ConversionConfig.IntegerEncodingOption.AUTO;
    configs.put(
        "default",
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(false)
            .useFSST(false)
            .useMortonEncoding(true)
            .preTessellatePolygons(false)
            .outlineFeatureTableNames(outlineNames)
            .integerEncoding(auto)
            .build());
    configs.put(
        "fastpfor",
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(true)
            .useFSST(false)
            .useMortonEncoding(true)
            .preTessellatePolygons(false)
            .outlineFeatureTableNames(outlineNames)
            .integerEncoding(auto)
            .build());
    configs.put(
        "fsst",
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(false)
            .useFSST(true)
            .useMortonEncoding(true)
            .preTessellatePolygons(false)
            .outlineFeatureTableNames(outlineNames)
            .integerEncoding(auto)
            .build());
    configs.put(
        "fastpfor-fsst",
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(true)
            .useFSST(true)
            .useMortonEncoding(true)
            .preTessellatePolygons(false)
            .outlineFeatureTableNames(outlineNames)
            .integerEncoding(auto)
            .build());
    return configs;
  }

  private static void exportMltFixture(
      String testCase, MapboxVectorTile tile, ConversionConfig config, String outputDir)
      throws IOException {
    var metadata = MltConverter.createTilesetMetadata(tile, Map.of(), config.getIncludeIds());
    byte[] mltData = MltConverter.convertMvt(tile, metadata, config, null);
    var outputPath = Paths.get(outputDir, testCase + ".mlt");
    Files.createDirectories(outputPath.getParent());
    Files.write(outputPath, mltData);
    var decodedTile = MltDecoder.decodeMlTile(mltData);
    String jsonOutput = CliUtil.printMLT(decodedTile);
    var jsonOutputPath = Paths.get(outputDir, testCase + ".json");
    Files.write(jsonOutputPath, jsonOutput.getBytes(StandardCharsets.UTF_8));
  }

  private static void generateMltFixtures() throws IOException {
    Files.createDirectories(Paths.get(OUTPUT_DIR));
    var tiles = generateTiles();
    var configs = generateConversionConfigs();
    exportMltFixture("points-default", tiles.get("points"), configs.get("default"), OUTPUT_DIR);
    exportMltFixture("points-fastpfor", tiles.get("points"), configs.get("fastpfor"), OUTPUT_DIR);
    exportMltFixture("polygons-default", tiles.get("polygons"), configs.get("default"), OUTPUT_DIR);
    exportMltFixture("mixed-fsst", tiles.get("mixed"), configs.get("fsst"), OUTPUT_DIR);
  }

  public static void main(String[] args) throws IOException {
    generateMltFixtures();
  }

  @Test
  @Disabled("Only for generating fixtures")
  void testGenerateMltFixtures() throws Exception {
    generateMltFixtures();
    var cases =
        new String[] {"points-default", "points-fastpfor", "polygons-default", "mixed-fsst"};
    for (var name : cases) {
      assertTrue(Files.isRegularFile(Paths.get(OUTPUT_DIR, name + ".mlt")), name + ".mlt");
      assertTrue(Files.isRegularFile(Paths.get(OUTPUT_DIR, name + ".json")), name + ".json");
    }
  }
}
