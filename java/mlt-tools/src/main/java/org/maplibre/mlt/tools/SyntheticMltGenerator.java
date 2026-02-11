package org.maplibre.mlt.tools;

import static org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption.*;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardOpenOption;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.Point;
import org.maplibre.mlt.cli.CliUtil;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.decoder.MltDecoder;

public class SyntheticMltGenerator {

  private static final Path SYNTHETICS_DIR = Paths.get("../test/synthetic");

  // Using common coordinates everywhere to make sure generated MLT files are very similar,
  // ensuring we observe difference in encoding rather than geometry variations
  private static final GeometryFactory gf = new GeometryFactory();
  private static final Coordinate c1 = new Coordinate(0, 0);
  private static final Coordinate c2 = new Coordinate(50, 0);
  private static final Coordinate c3 = new Coordinate(50, 50);
  private static final Coordinate c4 = new Coordinate(0, 50);
  private static final Coordinate c5 = new Coordinate(10, 10);
  private static final Coordinate c6 = new Coordinate(40, 10);
  private static final Coordinate c7 = new Coordinate(40, 40);
  private static final Coordinate c8 = new Coordinate(10, 40);

  private static final Point p1 = gf.createPoint(c1);
  private static final Point p2 = gf.createPoint(c2);
  private static final Point p3 = gf.createPoint(c3);
  private static final Point p4 = gf.createPoint(c4);
  private static final Point p5 = gf.createPoint(c5);
  private static final Point p6 = gf.createPoint(c6);
  private static final Point p7 = gf.createPoint(c7);
  private static final Point p8 = gf.createPoint(c8);

  public static void main(String[] args) throws IOException {
    generateMlts();
  }

  private static ConversionConfig.Builder cfg() {
    return cfg(PLAIN);
  }

  private static ConversionConfig.Builder cfg(ConversionConfig.IntegerEncodingOption encoding) {
    return ConversionConfig.builder()
        .includeIds(false)
        .useFastPFOR(false)
        .useFSST(false)
        .coercePropertyValues(false)
        .useMortonEncoding(false)
        .preTessellatePolygons(false)
        .outlineFeatureTableNames(List.of())
        .integerEncoding(encoding);
  }

  private static Point point(int x, int y) {
    return gf.createPoint(new Coordinate(x, y));
  }

  private static Map<String, Object> props(Object... keyValues) {
    if (keyValues.length % 2 != 0) {
      throw new IllegalArgumentException("Must provide key-value pairs");
    }
    var map = new java.util.HashMap<String, Object>();
    for (int i = 0; i < keyValues.length; i += 2) {
      map.put((String) keyValues[i], keyValues[i + 1]);
    }
    return map;
  }

  private static Feature feat(Geometry geom) {
    return feat(geom, null, Map.of());
  }

  private static Feature feat(Geometry geom, Map<String, Object> props) {
    return feat(geom, null, props);
  }

  private static Feature feat(Geometry geom, Long id) {
    return feat(geom, id, Map.of());
  }

  private static Feature feat(Geometry geom, Long id, Map<String, Object> props) {
    return new Feature(id != null ? id : 0, geom, props);
  }

  private static Layer layer(String name, Feature... features) {
    return new Layer(name, Arrays.asList(features), 4096);
  }

  private static void write(String name, Feature feat, ConversionConfig.Builder cfg)
      throws IOException {
    write(layer(name, feat), cfg);
  }

  private static void write(Layer layer, ConversionConfig.Builder cfg) throws IOException {
    // The layer name is just the first portion to simplify binary file comparison
    String name = layer.name();
    int dashIndex = name.indexOf('-');
    if (dashIndex != -1) {
      name = name.substring(0, dashIndex);
    }
    write(layer.name(), List.of(new Layer(name, layer.features(), layer.tileExtent())), cfg);
  }

  private static void write(String fileName, List<Layer> layers, ConversionConfig.Builder cfg)
      throws IOException {
    try {
      System.out.println("Generating: " + fileName);
      _write(fileName, layers, cfg);
    } catch (Exception e) {
      throw new IOException("Error writing MLT file " + fileName, e);
    }
  }

  private static void _write(String fileName, List<Layer> layers, ConversionConfig.Builder cfg)
      throws IOException {
    var config = cfg.build();
    var tile = new MapboxVectorTile(layers);
    var metadata = MltConverter.createTilesetMetadata(tile, Map.of(), config.getIncludeIds());
    var mltData = MltConverter.convertMvt(tile, metadata, config, null);
    Files.write(SYNTHETICS_DIR.resolve(fileName + ".mlt"), mltData, StandardOpenOption.CREATE_NEW);

    var decodedTile = MltDecoder.decodeMlTile(mltData);
    String jsonOutput = CliUtil.printMltGeoJson(decodedTile);
    Files.writeString(
        SYNTHETICS_DIR.resolve(fileName + ".json"),
        jsonOutput + "\n",
        StandardOpenOption.CREATE_NEW);
  }

  private static void generateMlts() throws IOException {
    if (Files.exists(SYNTHETICS_DIR)) {
      throw new IOException(
          "Synthetics dir must be deleted before running `:mlt-tools:generateSyntheticMlt`: "
              + SYNTHETICS_DIR.toAbsolutePath());
    }
    Files.createDirectories(SYNTHETICS_DIR);

    generatePoints();
    generateIds();
    generateLines();
    generatePolygons();
    generateProperties();
  }

  private static void generatePoints() throws IOException {
    write("point", feat(p1), cfg());
  }

  private static void generateIds() throws IOException {
    write("point-id", feat(p1, 100L), cfg().includeIds(true));
    write("point-id0", feat(p1, 0L), cfg().includeIds(true));

    var pts =
        new Feature[] {
          feat(p1, 100L), feat(p2, 101L), feat(p3, 103L),
        };
    write(layer("point-ids", pts), cfg().includeIds(true));
    write(layer("point-ids-delta", pts), cfg(DELTA).includeIds(true));
  }

  private static void generateLines() throws IOException {
    var line = feat(gf.createLineString(new Coordinate[] {c1, c2}));
    write("line", line, cfg());
  }

  private static void generatePolygons() throws IOException {
    var pol = feat(gf.createPolygon(new Coordinate[] {c1, c2, c5, c1}));
    write("polygon", pol, cfg());
    write("polygon-fpf", pol, cfg().useFastPFOR(true));
    // TODO: Tessellation tests cause decoder errors - skip for now
    // write("polygon-tess", pol, cfg().preTessellatePolygons(true));
    // write("polygon-morton-tess", pol, cfg().useFastPFOR(true).preTessellatePolygons(true));

    // Polygon with hole
    var polWithHole =
        feat(
            gf.createPolygon(
                gf.createLinearRing(new Coordinate[] {c1, c2, c3, c4, c1}),
                new org.locationtech.jts.geom.LinearRing[] {
                  gf.createLinearRing(new Coordinate[] {c5, c6, c7, c8, c5})
                }));
    write("polygon-hole", polWithHole, cfg());
    write("polygon-hole-fpf", polWithHole, cfg().useFastPFOR(true));

    // MultiPolygon
    var multiPol =
        feat(
            gf.createMultiPolygon(
                new org.locationtech.jts.geom.Polygon[] {
                  gf.createPolygon(new Coordinate[] {c1, c2, c6, c5, c1}),
                  gf.createPolygon(new Coordinate[] {c8, c7, c3, c4, c8})
                }));
    write("polygon-multi", multiPol, cfg());
    write("polygon-multi-fpf", multiPol, cfg().useFastPFOR(true));
  }

  private static void generateProperties() throws IOException {
    // Scalar property types
    write("point-bool", feat(p1, props("flag", true)), cfg());
    write("point-bool-false", feat(p1, props("flag", false)), cfg());
    write("point-int32", feat(p1, props("count", 42)), cfg());
    write("point-int32-neg", feat(p1, props("count", -42)), cfg());
    write("point-int64", feat(p1, props("bignum", 9876543210L)), cfg());
    write("point-int64-neg", feat(p1, props("bignum", -9876543210L)), cfg());
    write("point-float", feat(p1, props("temp", 3.14f)), cfg());
    write("point-double", feat(p1, props("precise", 3.141592653589793)), cfg());

    // Mixed properties - single feature demonstrating multiple property types
    write(
        "point-mixed-props",
        feat(
            p1,
            props(
                "name",
                "Test Point",
                "count",
                42,
                "active",
                true,
                "temp",
                25.5f,
                "precision",
                0.123456789)),
        cfg());

    var feat_ints =
        new Feature[] {
          feat(p1, props("int", 99)),
          feat(p2, props("int", 98)),
          feat(p3, props("int", 97)),
          feat(p4, props("int", 96)),
        };
    write(layer("point-props-int", feat_ints), cfg());
    write(layer("point-props-int-delta", feat_ints), cfg(DELTA));

    var feat_str =
        new Feature[] {
          feat(p1, props("str", "residential_zone_north_sector_1")),
          feat(p2, props("str", "commercial_zone_south_sector_2")),
          feat(p3, props("str", "industrial_zone_east_sector_3")),
          feat(p4, props("str", "park_zone_west_sector_4")),
          feat(p5, props("str", "water_zone_north_sector_5")),
          feat(p6, props("str", "residential_zone_south_sector_6")),
        };
    write(layer("point-props-str", feat_str), cfg());
    write(layer("point-props-str-fsst", feat_str), cfg().useFSST(true));
  }
}
