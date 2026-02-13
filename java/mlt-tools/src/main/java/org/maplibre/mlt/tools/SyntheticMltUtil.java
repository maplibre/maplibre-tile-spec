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
import org.locationtech.jts.geom.*;
import org.maplibre.mlt.cli.CliUtil;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.decoder.MltDecoder;

/** Utility helpers for synthetic MLT generation. */
class SyntheticMltUtil {

  static final Path SYNTHETICS_DIR = Paths.get("../test/synthetic/0x01");

  // Using common coordinates everywhere to make sure generated MLT files are very similar,
  // ensuring we observe difference in encoding rather than geometry variations
  static final GeometryFactory gf = new GeometryFactory();
  static final Coordinate c1 = new Coordinate(0, 0);
  static final Coordinate c2 = new Coordinate(50, 0);
  static final Coordinate c3 = new Coordinate(50, 50);
  static final Coordinate c4 = new Coordinate(0, 50);
  static final Coordinate c5 = new Coordinate(10, 10);
  static final Coordinate c6 = new Coordinate(40, 10);
  static final Coordinate c7 = new Coordinate(40, 40);
  static final Coordinate c8 = new Coordinate(10, 40);

  static final Point p1 = gf.createPoint(c1);
  static final Point p2 = gf.createPoint(c2);
  static final Point p3 = gf.createPoint(c3);
  static final Point p4 = gf.createPoint(c4);
  static final Point p5 = gf.createPoint(c5);
  static final Point p6 = gf.createPoint(c6);

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
      this.coercePropertyValues(true);
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
  }

  static Cfg cfg() {
    return cfg(PLAIN);
  }

  static Cfg cfg(ConversionConfig.IntegerEncodingOption encoding) {
    var c = new Cfg();
    c.includeIds(false);
    c.useFastPFOR(false);
    c.useFSST(false);
    c.coercePropertyValues(false);
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

  static Map<String, Object> props(Object... keyValues) {
    if (keyValues.length % 2 != 0) {
      throw new IllegalArgumentException("Must provide key-value pairs");
    }
    var map = new java.util.HashMap<String, Object>();
    for (int i = 0; i < keyValues.length; i += 2) {
      map.put((String) keyValues[i], keyValues[i + 1]);
    }
    return map;
  }

  static Feature feat(Geometry geom) {
    return feat(geom, null, Map.of());
  }

  static Feature feat(Geometry geom, Map<String, Object> props) {
    return feat(geom, null, props);
  }

  static Feature feat(Geometry geom, Long id) {
    return feat(geom, id, Map.of());
  }

  static Feature feat(Geometry geom, Long id, Map<String, Object> props) {
    return new Feature(id != null ? id : 0, geom, props);
  }

  static Layer layer(String name, Feature... features) {
    return new Layer(name, Arrays.asList(features), 4096);
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
      var config = cfg.build();
      var tile = new MapboxVectorTile(layers);
      var metadata = MltConverter.createTilesetMetadata(tile, Map.of(), config.getIncludeIds());
      var mltData = MltConverter.convertMvt(tile, metadata, config, null);
      Files.write(
          SYNTHETICS_DIR.resolve(fileName + ".mlt"), mltData, StandardOpenOption.CREATE_NEW);

      var decodedTile = MltDecoder.decodeMlTile(mltData);
      String jsonOutput = CliUtil.printMltGeoJson(decodedTile);
      Files.writeString(
          SYNTHETICS_DIR.resolve(fileName + ".json"),
          jsonOutput + "\n",
          StandardOpenOption.CREATE_NEW);
    } catch (Exception e) {
      throw new IOException("Error writing MLT file " + fileName, e);
    }
  }
}
