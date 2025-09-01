package org.maplibre.mlt;

import static org.maplibre.mlt.TestSettings.ID_REASSIGNABLE_MVT_LAYERS;

import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.FeatureTableOptimizations;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.RenderingOptimizedConversionConfig;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;
import java.io.Closeable;
import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.*;
import java.util.stream.Collectors;
import org.apache.commons.lang3.tuple.Triple;
import org.jetbrains.annotations.NotNull;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.Polygon;

public class MltGenerator {
  private static final int MIN_ZOOM = 0;
  private static final int MAX_ZOOM = 2;
  private static final String TILESET_METADATA_FILE_NAME = "tileset.pbf";

  private static final String MVT_SPECIFIC_TILES_SOURCE_MBTILES = "";
  private static final String MVT_SPECIFIC_TILES_SOURCE_DIR =
      "..\\ts\\test\\data\\omt\\optimized\\mvt";
  private static final String MLT_SPECIFIC_TILES_OUTPUT_DIR =
      "..\\ts\\test\\data\\omt\\optimized\\mlt\\optimized";
  private static final String MVT_SPECIFIC_TILES_OUTPUT_DIR =
      "..\\ts\\test\\data\\omt\\optimized\\mvt";

  private static final String MBTILES_FILE = "";
  private static final String MVT_OUTPUT_DIR = "..\\test\\data\\optimized\\omt\\mvt";
  private static final String MLT_OUTPUT_DIR = "..\\test\\data\\optimized\\omt\\mlt\\plain";

  // TestUtils.Optimization OPTIMIZATION = TestUtils.Optimization.IDS_REASSIGNED;
  // TestUtils.Optimization OPTIMIZATION = TestUtils.Optimization.SORTED;
  TestUtils.Optimization OPTIMIZATION = TestUtils.Optimization.NONE;
  protected static final Optional<List<ColumnMapping>> COLUMN_MAPPINGS =
      Optional.of(List.of(new ColumnMapping("name", "_", true)));
  boolean USE_ADVANCED_ENCODINGS = false;
  boolean USE_POLYGON_TESSELLATION = false;
  boolean USE_MORTON_ENCODING = false;
  private static final List<String> OUTLINE_POLYGON_FEATURE_TABLE_NAMES = List.of("building");

  @Test
  @Disabled
  public void generateMltTileset() throws IOException, SQLException, ClassNotFoundException {
    var mbTilesFilename = "jdbc:sqlite:" + MBTILES_FILE;
    var repo = new MbtilesRepsitory(mbTilesFilename, MIN_ZOOM, MAX_ZOOM);

    var tileMetadata =
        writeTileSetMetadata(
            MltConverter.createTilesetMetadata(repo, COLUMN_MAPPINGS, true), MLT_OUTPUT_DIR);

    var optimizations = getOptimizations();

    for (var mvTile : repo) {
      var tileId = mvTile.tileId();
      try {
        var mlTile = convertMvtToMlt(optimizations, true, mvTile, tileMetadata);

        var z = tileId.getLeft();
        var x = tileId.getMiddle();
        var y = (int) Math.pow(2, z) - tileId.getRight() - 1;
        writeTile(mlTile, MLT_OUTPUT_DIR, ".mlt", x, y, z);
        var rawMvTile = repo.getRawTile(tileId);
        writeTile(rawMvTile, MVT_OUTPUT_DIR, ".mvt", x, y, z);
      } catch (Exception e) {
        System.out.println("Error while processing tile " + tileId);
        e.printStackTrace();
      }
    }
  }

  @Test
  @Disabled
  public void generateSpecificMlTiles() throws IOException {
    var mvtFileNames =
        Files.list(Paths.get(MVT_SPECIFIC_TILES_SOURCE_DIR))
            .filter(file -> !Files.isDirectory(file))
            .map(Path::getFileName)
            .toList();

    var fullMvtFileNames =
        mvtFileNames.stream()
            .map(f -> Path.of(MVT_SPECIFIC_TILES_SOURCE_DIR, f.toString()))
            .toList();
    var mvTiles =
        fullMvtFileNames.stream()
            .map(
                f -> {
                  try {
                    return MvtUtils.decodeMvt(Files.readAllBytes(f), COLUMN_MAPPINGS);
                  } catch (IOException e) {
                    throw new RuntimeException(e);
                  }
                })
            .collect(Collectors.toList());

    var tileMetadata =
        writeTileSetMetadata(
            MltConverter.createTilesetMetadata(mvTiles, COLUMN_MAPPINGS, true),
            MLT_SPECIFIC_TILES_OUTPUT_DIR);

    var optimizations = getOptimizations();

    for (var tileName : mvtFileNames) {
      var mvt = Files.readAllBytes(Path.of(MVT_SPECIFIC_TILES_SOURCE_DIR, tileName.toString()));
      var mvTile = MvtUtils.decodeMvt(mvt, COLUMN_MAPPINGS);
      try {
        var mlTile = convertMvtToMlt(optimizations, USE_POLYGON_TESSELLATION, mvTile, tileMetadata);

        var mltFilename = tileName.toString().replace(".mvt", ".mlt");
        Files.write(Path.of(MLT_SPECIFIC_TILES_OUTPUT_DIR, mltFilename), mlTile);
      } catch (Exception e) {
        System.out.println("Error while processing tile " + tileName);
        e.printStackTrace();
      }
    }
  }

  @Test
  @Disabled
  public void generateSpecificMlTilesFromMbtiles()
      throws IOException, SQLException, ClassNotFoundException {
    var repo = new MbtilesRepsitory("jdbc:sqlite:" + MVT_SPECIFIC_TILES_SOURCE_MBTILES, 0, 14);
    var mvTiles = repo.getLargestTilesPerZoom();

    var decodedMvTiles = mvTiles.stream().map(Triple::getMiddle).collect(Collectors.toList());
    var tileMetadata =
        writeTileSetMetadata(
            MltConverter.createTilesetMetadata(decodedMvTiles, COLUMN_MAPPINGS, true),
            MLT_SPECIFIC_TILES_OUTPUT_DIR);

    var optimizations = getOptimizations();

    for (var mvTile : mvTiles) {
      try {
        var mlTile =
            convertMvtToMlt(
                optimizations, USE_POLYGON_TESSELLATION, mvTile.getMiddle(), tileMetadata);

        var tileId = mvTile.getRight();
        var tileName = tileId.getLeft() + "_" + tileId.getMiddle() + "_" + tileId.getRight();
        Files.write(Path.of(MLT_SPECIFIC_TILES_OUTPUT_DIR, tileName + ".mlt"), mlTile);
        Files.write(Path.of(MVT_SPECIFIC_TILES_OUTPUT_DIR, tileName + ".mvt"), mvTile.getLeft());
      } catch (Exception e) {
        System.out.println("Error while processing tile " + mvTile);
        e.printStackTrace();
      }
    }
  }

  private Map<String, FeatureTableOptimizations> getOptimizations() {
    var allowSorting = OPTIMIZATION == TestUtils.Optimization.SORTED;
    var featureTableOptimization =
        new FeatureTableOptimizations(allowSorting, false, COLUMN_MAPPINGS);
    var optimizations =
        TestSettings.OPTIMIZED_MVT_LAYERS.stream()
            .collect(Collectors.toMap(l -> l, l -> featureTableOptimization));

    /* Only regenerate the ids for specific layers when the column is not sorted for comparison reasons */
    if (OPTIMIZATION == TestUtils.Optimization.IDS_REASSIGNED) {
      for (var reassignableLayer : ID_REASSIGNABLE_MVT_LAYERS) {
        optimizations.put(
            reassignableLayer, new FeatureTableOptimizations(false, true, COLUMN_MAPPINGS));
      }
    }
    return optimizations;
  }

  private byte[] convertMvtToMlt(
      Map<String, FeatureTableOptimizations> optimizations,
      boolean preTessellatePolygons,
      MapboxVectorTile mvTile,
      MltTilesetMetadata.TileSetMetadata tileMetadata)
      throws IOException {
    var config =
        USE_POLYGON_TESSELLATION
            ? new RenderingOptimizedConversionConfig(
                true,
                USE_ADVANCED_ENCODINGS,
                optimizations,
                preTessellatePolygons,
                OUTLINE_POLYGON_FEATURE_TABLE_NAMES)
            : new ConversionConfig(
                true, USE_ADVANCED_ENCODINGS, optimizations, USE_MORTON_ENCODING);
    return MltConverter.convertMvt(mvTile, config, tileMetadata);
  }

  private static MltTilesetMetadata.TileSetMetadata writeTileSetMetadata(
      MltTilesetMetadata.TileSetMetadata tilesetMetadata, String mltOutputDir) throws IOException {
    var outputMetadataPath = Paths.get(mltOutputDir, TILESET_METADATA_FILE_NAME);
    tilesetMetadata.writeTo(Files.newOutputStream(outputMetadataPath));
    return tilesetMetadata;
  }

  private static void writeTile(
      byte[] tile, String outDir, String tileExtension, int x, int y, int z) throws IOException {
    var path = Paths.get(outDir, Integer.toString(z), Integer.toString(x), Integer.toString(y));
    path.toFile().mkdirs();
    var mltFilename = path.resolve(y + tileExtension);
    System.out.println("Writing tile to " + mltFilename);
    Files.write(mltFilename, tile);
    var compressedTile = EncodingUtils.gzip(tile);
    var compressedTileName = path.resolve(y + tileExtension + ".gz");
    Files.write(compressedTileName, compressedTile);
  }

  private Layer createDebugLayer() {
    var layer = new Layer("debug", new ArrayList<>(), 4096);
    var features = layer.features();
    var geometryFactory = new GeometryFactory();
    var shell1 =
        geometryFactory.createLinearRing(
            new Coordinate[] {
              new Coordinate(100, 100),
              new Coordinate(1800, 100),
              new Coordinate(1800, 1800),
              new Coordinate(100, 1800),
              new Coordinate(400, 1000),
              new Coordinate(100, 100)
            });
    /*var hole1 = geometryFactory.createLinearRing(new Coordinate[]{
            new Coordinate(500, 500),
            new Coordinate(500, 1000),
            new Coordinate(1000, 1000),
            new Coordinate(1000, 500),
            new Coordinate(700, 500),
            new Coordinate(600, 500),
            new Coordinate(500, 500)
    });
    var hole2 = geometryFactory.createLinearRing(new Coordinate[]{
            new Coordinate(1200, 700),
            new Coordinate(1200, 1400),
            new Coordinate(1700, 1400),
            new Coordinate(1500, 1000),
            new Coordinate(1200, 700)
    });*/
    var shell2 =
        geometryFactory.createLinearRing(
            new Coordinate[] {
              new Coordinate(2100, 100),
              new Coordinate(3800, 100),
              new Coordinate(3800, 3800),
              new Coordinate(2100, 3800),
              new Coordinate(2100, 100)
            });
    /*var hole4 = geometryFactory.createLinearRing(new Coordinate[]{
            new Coordinate(2500, 500),
            new Coordinate(2500, 3200),
            new Coordinate(3200, 3200),
            new Coordinate(3200, 500),
            new Coordinate(2500, 500)
    });*/
    var shell4 =
        geometryFactory.createLinearRing(
            new Coordinate[] {
              new Coordinate(2100, 3810),
              new Coordinate(2100, 3990),
              new Coordinate(3950, 3990),
              new Coordinate(3990, 3810),
              new Coordinate(3000, 3810),
              new Coordinate(2100, 3810)
            });
    var polygon1 = geometryFactory.createPolygon(shell1);
    var polygon2 = geometryFactory.createPolygon(shell2);
    var multiPolygon = geometryFactory.createMultiPolygon(new Polygon[] {polygon1, polygon2});

    Map<String, Object> properties = Map.of("key", "test");
    var feature = new Feature(1, multiPolygon, properties);
    features.add(feature);

    var polygon4 = geometryFactory.createPolygon(shell4);
    Map<String, Object> properties2 = Map.of("key", "test");
    var feature2 = new Feature(1, polygon4, properties);
    features.add(feature2);

    return layer;
  }
}

class MbtilesRepsitory implements Iterable<MapboxVectorTile>, Closeable {
  private static final String TILE_TABLE_NAME = "tiles";
  private final Connection connection;
  private final Statement statement;
  protected final int minZoom;
  protected final int maxZoom;

  MbtilesRepsitory(String url, int minZoom, int maxZoom)
      throws ClassNotFoundException, SQLException {
    Class.forName("org.sqlite.JDBC");
    this.connection = DriverManager.getConnection(url);
    this.statement = this.connection.createStatement();
    this.minZoom = minZoom;
    this.maxZoom = maxZoom;
  }

  protected MapboxVectorTile getTile(Triple<Integer, Integer, Integer> tileId) {
    try {
      var rs =
          statement.executeQuery(
              String.format(
                  "SELECT * FROM %s WHERE zoom_level = %d AND"
                      + " tile_column = %d AND tile_row = %d;",
                  TILE_TABLE_NAME, tileId.getLeft(), tileId.getMiddle(), tileId.getRight()));

      rs.next();
      InputStream in = rs.getBinaryStream("tile_data");
      byte[] mvt = new byte[in.available()];
      in.read(mvt);

      var uncompressedMvt = EncodingUtils.unzip(mvt);
      return MvtUtils.decodeMvt(uncompressedMvt, MltGenerator.COLUMN_MAPPINGS);
    } catch (SQLException | IOException e) {
      throw new RuntimeException(e);
    }
  }

  protected byte[] getRawTile(Triple<Integer, Integer, Integer> tileId)
      throws SQLException, IOException {
    var rs =
        statement.executeQuery(
            String.format(
                "SELECT * FROM %s WHERE zoom_level = %d AND"
                    + " tile_column = %d AND tile_row = %d;",
                TILE_TABLE_NAME, tileId.getLeft(), tileId.getMiddle(), tileId.getRight()));

    rs.next();
    InputStream in = rs.getBinaryStream("tile_data");
    byte[] mvt = new byte[in.available()];
    in.read(mvt);

    return EncodingUtils.unzip(mvt);
  }

  public List<Triple<byte[], MapboxVectorTile, Triple<Integer, Integer, Integer>>>
      getLargestTilesPerZoom() {
    try {
      var mvTiles =
          new ArrayList<Triple<byte[], MapboxVectorTile, Triple<Integer, Integer, Integer>>>();
      for (var zoom = 0; zoom <= maxZoom; zoom++) {
        var rs =
            statement.executeQuery(
                String.format(
                    "SELECT * FROM %s WHERE zoom_level = %d "
                        + "ORDER BY LENGTH(tile_data) DESC LIMIT 1;",
                    TILE_TABLE_NAME, zoom));

        rs.next();
        InputStream in = rs.getBinaryStream("tile_data");
        byte[] mvt = new byte[in.available()];
        in.read(mvt);
        var x = rs.getInt("tile_column");
        var y = rs.getInt("tile_row");

        var uncompressedMvt = EncodingUtils.unzip(mvt);
        var decodedMvt = MvtUtils.decodeMvt(uncompressedMvt, MltGenerator.COLUMN_MAPPINGS);
        var tileId = Triple.of(zoom, x, y);
        mvTiles.add(Triple.of(uncompressedMvt, decodedMvt, tileId));
      }

      return mvTiles;
    } catch (SQLException | IOException e) {
      throw new RuntimeException(e);
    }
  }

  public Queue<Triple<Integer, Integer, Integer>> getTileIds() {
    try {
      // TODO: read in batches to scale also for a planet-scale tileset
      var rs =
          statement.executeQuery(
              String.format(
                  "SELECT * FROM %s WHERE zoom_level >= %d AND zoom_level <= %d",
                  TILE_TABLE_NAME, minZoom, maxZoom));
      var tileIds = new LinkedList<Triple<Integer, Integer, Integer>>();
      while (rs.next()) {
        int z = rs.getInt("zoom_level");
        int x = rs.getInt("tile_column");
        int y = rs.getInt("tile_row");
        var tileId = Triple.of(z, x, y);
        tileIds.add(tileId);
      }

      return tileIds;
    } catch (SQLException e) {
      throw new RuntimeException(e);
    }
  }

  public void close() {
    try {
      statement.close();
      connection.close();
    } catch (SQLException e) {
      throw new RuntimeException(e);
    }
  }

  @NotNull
  @Override
  public Iterator<MapboxVectorTile> iterator() {
    return new MbtilesIterator();
  }

  private class MbtilesIterator implements Iterator<MapboxVectorTile> {
    private Queue<Triple<Integer, Integer, Integer>> tileIds;

    @Override
    public boolean hasNext() {
      if (tileIds == null) {
        tileIds = getTileIds();
      }

      return !tileIds.isEmpty();
    }

    @Override
    public MapboxVectorTile next() {
      var tileId = tileIds.poll();
      var tile = getTile(tileId);
      tile.setTileId(tileId);
      return tile;
    }
  }
}
