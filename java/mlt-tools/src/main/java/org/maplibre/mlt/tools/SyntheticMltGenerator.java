package org.maplibre.mlt.tools;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.GeometryFactory;
import org.maplibre.mlt.cli.CliUtil;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.decoder.MltDecoder;

public class SyntheticMltGenerator {

  private static final int TILE_EXTENT = 4096;
  private static final String OUTPUT_DIR = "../test/synthetic";

  // Object generation
  private static Feature generateSinglePoint(GeometryFactory gf, long id, double x, double y) {
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
    var f1 = generateSinglePoint(gf, nextId[0]++, 10.0, 50.0);
    var f2 = generateSinglePoint(gf, nextId[0]++, 20.0, 40.0);
    return new Layer("points", List.of(f1, f2), TILE_EXTENT);
  }

  private static Layer generatePolygonsLayer(GeometryFactory gf, long[] nextId) {
    var f1 = generateSimplePolygon(gf, nextId[0]++, 5.0, 45.0, 5.0);
    var f2 = generateSimplePolygon(gf, nextId[0]++, 15.0, 35.0, 5.0);
    return new Layer("polygons", List.of(f1, f2), TILE_EXTENT);
  }

  // MapboxVectorTile tile generation
  // Note: MapboxVectorTile is in-memory object structure (it is not MVT file)
  private static MapboxVectorTile generatePointsTile() {
    var gf = new GeometryFactory();
    long[] nextId = {1};
    var layer = generatePointsLayer(gf, nextId);
    return new MapboxVectorTile(List.of(layer));
  }

  private static MapboxVectorTile generatePolygonsTile() {
    var gf = new GeometryFactory();
    long[] nextId = {1};
    var layer = generatePolygonsLayer(gf, nextId);
    return new MapboxVectorTile(List.of(layer));
  }

  private static MapboxVectorTile generateMixedGeometriesTile() {
    var gf = new GeometryFactory();
    long[] nextId = {1};
    var pointsLayer = generatePointsLayer(gf, nextId);
    var polygonsLayer = generatePolygonsLayer(gf, nextId);
    return new MapboxVectorTile(List.of(pointsLayer, polygonsLayer));
  }

  private static void writeMltFixture(
      String outputName, MapboxVectorTile tileGeometry, ConversionConfig config)
      throws IOException {
    var metadata =
        MltConverter.createTilesetMetadata(tileGeometry, Map.of(), config.getIncludeIds());
    byte[] mltData = MltConverter.convertMvt(tileGeometry, metadata, config, null);
    var outputPath = Paths.get(OUTPUT_DIR, outputName + ".mlt");
    Files.createDirectories(outputPath.getParent());
    Files.write(outputPath, mltData);
    var decodedTile = MltDecoder.decodeMlTile(mltData);
    String jsonOutput = CliUtil.printMltGeoJson(decodedTile);
    var jsonOutputPath = Paths.get(OUTPUT_DIR, outputName + ".json");
    Files.write(jsonOutputPath, jsonOutput.getBytes(StandardCharsets.UTF_8));
  }

  private static void generateMltFixtures() throws IOException {
    Files.createDirectories(Paths.get(OUTPUT_DIR));

    // Common config parameters
    var outlineNames = List.<String>of();
    var plain = ConversionConfig.IntegerEncodingOption.PLAIN;
    var delta = ConversionConfig.IntegerEncodingOption.DELTA;
    var rle = ConversionConfig.IntegerEncodingOption.RLE;

    // Case 1
    writeMltFixture(
        "points-plain",
        generatePointsTile(),
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(false)
            .useFSST(false)
            .coercePropertyValues(false)
            .useMortonEncoding(false)
            .preTessellatePolygons(false)
            .outlineFeatureTableNames(outlineNames)
            .integerEncoding(plain)
            .build());

    // Case 2
    writeMltFixture(
        "polygons-fsst-delta",
        generatePolygonsTile(),
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(false)
            .useFSST(true)
            .coercePropertyValues(false)
            .useMortonEncoding(false)
            .preTessellatePolygons(false)
            .outlineFeatureTableNames(outlineNames)
            .integerEncoding(delta)
            .build());

    // Case 3
    writeMltFixture(
        "mixed-fastpfor-rle",
        generateMixedGeometriesTile(),
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(true)
            .useFSST(false)
            .coercePropertyValues(false)
            .useMortonEncoding(false)
            .preTessellatePolygons(false)
            .outlineFeatureTableNames(outlineNames)
            .integerEncoding(rle)
            .build());
  }

  public static void main(String[] args) throws IOException {
    generateMltFixtures();
  }
}
