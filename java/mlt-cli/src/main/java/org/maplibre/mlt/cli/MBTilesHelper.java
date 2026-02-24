package org.maplibre.mlt.cli;

import jakarta.annotation.Nullable;
import java.io.ByteArrayInputStream;
import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import org.apache.commons.lang3.mutable.MutableBoolean;
import org.imintel.mbtiles4j.MBTilesReadException;
import org.imintel.mbtiles4j.MBTilesReader;
import org.imintel.mbtiles4j.MBTilesWriteException;
import org.imintel.mbtiles4j.MBTilesWriter;
import org.imintel.mbtiles4j.Tile;
import org.jetbrains.annotations.NotNull;
import org.jspecify.annotations.NonNull;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.metadata.tileset.MltMetadata;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class MBTilesHelper extends ConversionHelper {
  static final String MetadataMIMEType = "application/vnd.maplibre-vector-tile";

  /// Encode the entire contents of an MBTile file of MVT tiles
  static boolean encodeMBTiles(
      @NotNull String inputMBTilesPath, @Nullable Path outputPath, @NotNull EncodeConfig config) {
    MBTilesReader mbTilesReader = null;
    final var success = new AtomicBoolean(true);
    try {
      mbTilesReader = new MBTilesReader(new File(inputMBTilesPath));

      // Remove any existing output file, as SQLite will add to it instead of replacing
      if (outputPath != null) {
        Files.deleteIfExists(outputPath);
      }

      // If no target file is specified, a temporary is used.
      // mbtiles4j blocks access to in-memory databases and built-in
      // temp file support, so we have to use its temp file support.
      var mbTilesWriter =
          (outputPath != null)
              ? new MBTilesWriter(outputPath.toFile())
              : new MBTilesWriter("encode");
      try {
        // Copy metadata from the input file.
        // We can't set the format, see the coda.
        final var metadata = mbTilesReader.getMetadata();
        mbTilesWriter.addMetadataEntry(metadata);

        final var pbMeta = new MltMetadata.TileSetMetadata();
        pbMeta.name = metadata.getTilesetName();
        pbMeta.attribution = metadata.getAttribution();
        pbMeta.description = metadata.getTilesetDescription();

        final var bounds = metadata.getTilesetBounds();
        pbMeta.bounds =
            List.of(bounds.getLeft(), bounds.getTop(), bounds.getRight(), bounds.getBottom());

        final var metadataJSON = MltConverter.createTilesetMetadataJSON(pbMeta);

        // Read everything on the main thread for simplicity.
        // Splitting reads by zoom level might improve performance.
        final var tiles = mbTilesReader.getTiles();
        final var tileCount = new AtomicLong(0);
        final var writerCapture = mbTilesWriter;
        try {
          while (tiles.hasNext()) {
            final var tile = tiles.next();
            try {
              if (!config.continueOnError() && !success.get()) {
                break;
              }

              final var x = tile.getColumn();
              final var y = tile.getRow();
              final var z = tile.getZoom();

              if (z < config.minZoom() || z > config.maxZoom()) {
                logger.trace("Skipping tile {}:{},{} outside zoom range", z, x, y);
                continue;
              }

              final var data = tile.getData().readAllBytes();

              config
                  .taskRunner()
                  .run(
                      () -> {
                        final var count = tileCount.incrementAndGet();
                        if (count % TILE_LOG_INTERVAL == 0) {
                          logger.debug("Processing tile {} : {}:{},{}", count, z, x, y);
                        }
                        if (!convertTile(config, data, writerCapture, tile)) {
                          success.set(false);
                        }
                      });
            } catch (IllegalArgumentException ex) {
              success.set(false);
              logger.error(
                  "Failed to convert tile {}:{},{}",
                  tile.getZoom(),
                  tile.getColumn(),
                  tile.getRow(),
                  ex);
            }
          }

          try {
            config.taskRunner().shutdown();
            config.taskRunner().awaitTermination();
          } catch (InterruptedException ex) {
            logger.error("Interrupted", ex);
            return false;
          }

          if (!config.continueOnError() && !success.get()) {
            return false;
          }

          final var dbFile = mbTilesWriter.close();
          if (outputPath == null) {
            dbFile.deleteOnExit();
          }
          mbTilesWriter = null;

          // mbtiles4j doesn't support types other than png and jpg,
          // so we have to set the format metadata the hard way.
          final var connectionString = "jdbc:sqlite:" + dbFile.getAbsolutePath();
          try (var connection = DriverManager.getConnection(connectionString)) {
            updateMetadata(config, connection, metadataJSON);
          }
        } finally {
          // mbtiles4j doesn't support `AutoCloseable`
          tiles.close();
        }
      } finally {
        if (mbTilesWriter != null) {
          final var file = mbTilesWriter.close();
          if (outputPath == null) {
            file.delete();
          }
        }
      }
    } catch (MBTilesReadException | IOException | MBTilesWriteException | SQLException ex) {
      success.set(false);
      logger.error("Failed to convert MBTiles file", ex);
    } finally {
      if (mbTilesReader != null) {
        mbTilesReader.close();
      }
    }
    return success.get();
  }

  private static void updateMetadata(
      @NonNull EncodeConfig config, @NonNull Connection connection, @NonNull String metadataJSON)
      throws SQLException {
    logger.debug("Updating metadata");
    logger.trace("Setting tile MIME type to '{}'", MetadataMIMEType);

    var sql = "UPDATE metadata SET value = ? WHERE name = ?";
    try (var statement = connection.prepareStatement(sql)) {
      statement.setString(1, MetadataMIMEType);
      statement.setString(2, "format");
      statement.execute();
    }

    // Put the global metadata in a custom metadata key.
    // Could also be in a custom key within the standard `json` entry...
    logger.trace("Adding tileset metadata JSON: {}", metadataJSON);
    sql = "INSERT OR REPLACE INTO metadata (name, value) VALUES (?, ?)";
    try (var statement = connection.prepareStatement(sql)) {
      statement.setString(1, "mln-json");
      statement.setString(2, metadataJSON);
      statement.execute();
    }

    vacuumDatabase(connection);
  }

  private static boolean convertTile(
      @NonNull EncodeConfig config,
      byte[] data,
      @NonNull MBTilesWriter writerCapture,
      @NonNull Tile tile) {
    try {
      final var x = tile.getColumn();
      final var y = tile.getRow();
      final var z = tile.getZoom();
      final var srcTileData = decompress(new ByteArrayInputStream(data));
      final var didCompress = new MutableBoolean(false);
      final var tileData =
          Encode.convertTile(
              x,
              y,
              z,
              srcTileData,
              config,
              Optional.of(DEFAULT_COMPRESSION_RATIO_THRESHOLD),
              Optional.of(DEFAULT_COMPRESSION_FIXED_THRESHOLD),
              didCompress);

      if (tileData != null) {
        synchronized (writerCapture) {
          writerCapture.addTile(tileData, z, x, y);
        }
      }
      return true;
    } catch (IOException | MBTilesWriteException ex) {
      logger.error(
          "Failed to convert tile {}:{},{}", tile.getZoom(), tile.getColumn(), tile.getRow(), ex);
    }
    return false;
  }

  private static final Logger logger = LoggerFactory.getLogger(MBTilesHelper.class);
}
