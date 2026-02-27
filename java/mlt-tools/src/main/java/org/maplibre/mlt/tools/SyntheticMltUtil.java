package org.maplibre.mlt.tools;

import static org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption.*;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardOpenOption;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import org.locationtech.jts.geom.*;
import org.maplibre.mlt.cli.JsonHelper;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.FeatureTableOptimizations;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.ColumnMappingConfig;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.decoder.MltDecoder;

/** Utility helpers for synthetic MLT generation. */
class SyntheticMltUtil {

  static final Path SYNTHETICS_DIR = Paths.get("../test/synthetic/0x01");

  // Using common coordinates everywhere to make sure generated MLT files are very similar,
  // ensuring we observe difference in encoding rather than geometry variations.
  // Use SRID 4326 for visualization - all coords are in positive longitude/latitude
  // range, but in reality the coords are in SRID=0 tile space.
  // Use coordinates X in 0..180, Y in 0..85 space to match lon/lat ranges and help QGIS vis.
  // Try to keep all values unique to simplify validation and debugging.
  // Use tiny tile extent 80 for most geometry tests to focus on encoding correctness.
  static final GeometryFactory gf = new GeometryFactory(new PrecisionModel(), 4326);
  static final Coordinate c0 = c(13, 42);
  // triangle 1, clockwise winding, X ends in 1, Y ends in 2
  static final Coordinate c1 = c(11, 52);
  static final Coordinate c2 = c(71, 72);
  static final Coordinate c3 = c(61, 22);
  // triangle 2, clockwise winding, X ends in 3, Y ends in 4
  static final Coordinate c21 = c(23, 34);
  static final Coordinate c22 = c(73, 4);
  static final Coordinate c23 = c(13, 24);
  // hole in triangle 1 with counter-clockwise winding
  static final Coordinate h1 = c(65, 66);
  static final Coordinate h2 = c(35, 56);
  static final Coordinate h3 = c(55, 36);

  static final Point p0 = gf.createPoint(c0);
  static final Point p1 = gf.createPoint(c1);
  static final Point p2 = gf.createPoint(c2);
  static final Point p3 = gf.createPoint(c3);
  // holes as points with same coordinates as the hole vertices
  static final Point ph1 = gf.createPoint(h1);
  static final Point ph2 = gf.createPoint(h2);
  static final Point ph3 = gf.createPoint(h3);

  static final LineString line1 = line(c1, c2, c3);
  static final LineString line2 = line(c21, c22, c23);
  static final Polygon poly1 = poly(c1, c2, c3, c1);
  static final Polygon poly2 = poly(c21, c22, c23, c21);
  static final Polygon poly1h = poly(ring(c1, c2, c3, c1), ring(h1, h2, h3, h1));

  /** Builder subclass with no-argument shorthand methods for common flags. */
  static class Cfg extends ConversionConfig.Builder {
    public Cfg ids() {
      this.includeIds(true);
      return this;
    }

    public Cfg fastPFOR() {
      this.useFastPFOR(true);
      return this;
    }

    public Cfg fsst() {
      this.useFSST(true);
      return this;
    }

    public Cfg coercePropValues() {
      this.mismatchPolicy(ConversionConfig.TypeMismatchPolicy.COERCE);
      return this;
    }

    public Cfg tessellate() {
      this.preTessellatePolygons(true);
      return this;
    }

    public Cfg morton() {
      this.useMortonEncoding(true);
      return this;
    }

    public Cfg filterInvert() {
      this.layerFilterInvert(true);
      return this;
    }

    /**
     * Enable shared dictionary encoding for properties with names starting with the given prefix
     * string.
     */
    public Cfg sharedDictPrefix(String prefix, String delimiter) {
      var mapping = new ColumnMapping(prefix, delimiter, true);
      var layerOpt = new FeatureTableOptimizations(false, false, List.of(mapping));

      var optimizationsMap = new HashMap<String, FeatureTableOptimizations>();
      optimizationsMap.put("layer1", layerOpt);
      this.optimizations(optimizationsMap);
      return this;
    }
  }

  static Cfg cfg() {
    return cfg(PLAIN);
  }

  static Cfg cfg(ConversionConfig.IntegerEncodingOption encoding) {
    var c = new Cfg();
    c.includeIds(false);
    c.useFastPFOR(false);
    c.useFSST(false);
    c.mismatchPolicy(ConversionConfig.TypeMismatchPolicy.FAIL);
    // c.optimizations(null); // Map<String, FeatureTableOptimizations>
    c.preTessellatePolygons(false);
    c.useMortonEncoding(false);
    // ALWAYS use outlines for synthetic tests. When tessellation is enabled, a polygon outline
    // also needs to be stored in the tile, or else only triangles would be stored and the
    // Java decoder (and other decoders) do not currently support triangle meshes without
    // corresponding outline polygons. Using "ALL" here ensures outlines are present for both
    // tessellated and non-tessellated cases in all synthetic test tiles.
    c.outlineFeatureTableNames(List.of("ALL"));
    // c.layerFilterPattern(null); // Pattern
    c.layerFilterInvert(false);
    c.integerEncoding(encoding);
    return c;
  }

  @SafeVarargs
  @SuppressWarnings("varargs")
  static <T> T[] array(T... elements) {
    return elements;
  }

  static Coordinate c(int x, int y) {
    return new Coordinate(x, y);
  }

  static LineString line(Coordinate... coords) {
    return gf.createLineString(coords);
  }

  static LinearRing ring(Coordinate... coords) {
    return gf.createLinearRing(coords);
  }

  static Polygon poly(Coordinate... coords) {
    return gf.createPolygon(coords);
  }

  static Polygon poly(LinearRing shell, LinearRing... holes) {
    return gf.createPolygon(shell, holes);
  }

  static MultiPoint multi(Point... pts) {
    return gf.createMultiPoint(pts);
  }

  static MultiPolygon multi(Polygon... polys) {
    return gf.createMultiPolygon(polys);
  }

  static MultiLineString multi(LineString... lines) {
    return gf.createMultiLineString(lines);
  }

  record KeyVal(String key, Object value) {}

  static KeyVal kv(String key, Object value) {
    return new KeyVal(key, value);
  }

  static Map<String, Object> prop(String key, Object value) {
    return Map.of(key, value);
  }

  static Map<String, Object> props(KeyVal... keyValues) {
    var map = new java.util.HashMap<String, Object>();
    for (var kv : keyValues) {
      map.put(kv.key, kv.value);
    }
    return map;
  }

  static Feature feat(Geometry geom) {
    return new Feature(geom, Map.of());
  }

  static Feature feat(Geometry geom, Map<String, Object> props) {
    return new Feature(geom, props);
  }

  /** for testing IDs - always use the same geometry */
  static Feature idFeat(long id) {
    return new Feature(id, p0, Map.of());
  }

  /** for testing IDs - simulate missing ID */
  static Feature idFeat() {
    return new Feature(p0, Map.of());
  }

  static Layer layer(String name, Feature... features) {
    return new Layer(name, Arrays.asList(features), 80);
  }

  static Layer layer(String name, int extent, Feature... features) {
    return new Layer(name, Arrays.asList(features), extent);
  }

  static void write(String name, Feature feat, ConversionConfig.Builder cfg) throws IOException {
    write(layer(name, feat), cfg);
  }

  static void write(Layer layer, ConversionConfig.Builder cfg) throws IOException {
    // layer names should be identical to reduce variability in generated MLT files
    // and ensure we observe differences in encoding rather than layer name variations
    write(layer.name(), List.of(new Layer("layer1", layer.features(), layer.tileExtent())), cfg);
  }

  static void write(String fileName, List<Layer> layers, ConversionConfig.Builder cfg)
      throws IOException {
    try {
      System.out.println("Generating: " + fileName);
      var mltFile = SYNTHETICS_DIR.resolve(fileName + ".mlt");
      Path jsonFile = SYNTHETICS_DIR.resolve(fileName + ".json");

      var config = cfg.build();
      var tile = new MapboxVectorTile(layers);

      // Extract column mappings from the config's optimizations
      final var columnMappings = new ColumnMappingConfig();
      if (config.getOptimizations() != null && !config.getOptimizations().isEmpty()) {
        var allColumnMappings =
            config.getOptimizations().values().stream()
                .flatMap(opt -> opt.columnMappings().stream())
                .toList();
        if (!allColumnMappings.isEmpty()) {
          columnMappings.put(Pattern.compile(".*"), allColumnMappings);
        }
      }

      var metadata =
          MltConverter.createTilesetMetadata(tile, columnMappings, config.getIncludeIds());
      var mlt = MltConverter.convertMvt(tile, metadata, config, null);
      Files.write(mltFile, mlt, StandardOpenOption.CREATE_NEW);

      final String json = JsonHelper.toGeoJson(MltDecoder.decodeMlTile(mlt)) + "\n";
      Files.writeString(jsonFile, json, StandardOpenOption.CREATE_NEW);
    } catch (Exception e) {
      throw new IOException("Error writing MLT file " + fileName, e);
    }
  }
}
