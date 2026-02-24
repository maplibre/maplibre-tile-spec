package org.maplibre.mlt.cli;

import com.google.gson.Gson;
import com.google.gson.JsonObject;
import com.google.gson.JsonSyntaxException;
import jakarta.annotation.Nullable;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.SQLException;
import java.util.Optional;
import java.util.concurrent.atomic.AtomicBoolean;
import org.apache.commons.lang3.mutable.MutableBoolean;
import org.jetbrains.annotations.NotNull;
import org.jspecify.annotations.NonNull;

public class OfflineDBHelper extends ConversionHelper {
  /// Encode the MVT tiles in an offline database file
  static boolean encodeOfflineDB(
      @NotNull Path inputPath, @Nullable Path outputPath, @NotNull EncodeConfig config)
      throws ClassNotFoundException {
    // Start with a copy of the source file so we don't have to rebuild the complex schema
    try {
      if (outputPath == null) {
        final var tempFile = File.createTempFile("encode-", "-db");
        outputPath = tempFile.toPath();
        tempFile.deleteOnExit();
      }
      if (config.verboseLevel() > 1) {
        System.err.printf("Copying source to %s%n", outputPath);
      }
      Files.copy(
          inputPath,
          outputPath,
          StandardCopyOption.REPLACE_EXISTING,
          StandardCopyOption.COPY_ATTRIBUTES);
    } catch (IOException ex) {
      System.err.println("ERROR: Failed to create target file: " + ex.getMessage());
      return false;
    }

    Class.forName("org.sqlite.JDBC");
    final var srcConnectionString = "jdbc:sqlite:" + inputPath.toAbsolutePath();
    final var dstConnectionString = "jdbc:sqlite:" + outputPath.toAbsolutePath();
    final var updateSql = "UPDATE tiles SET data = ?, compressed = ? WHERE id = ?";
    final var success = new AtomicBoolean(true);
    try (final var srcConnection = DriverManager.getConnection(srcConnectionString);
        final var dstConnection = DriverManager.getConnection(dstConnectionString)) {
      // Convert Tiles
      try (final var iterateStatement = srcConnection.createStatement();
          final var tileResults = iterateStatement.executeQuery("SELECT * FROM tiles");
          final var updateStatement = dstConnection.prepareStatement(updateSql)) {
        while (tileResults.next()) {
          if (!config.continueOnError() && !success.get()) {
            break;
          }

          // Read on the main thread. Could be split by zoom level, etc., if needed.
          final var uniqueID = tileResults.getLong("id");
          final var x = tileResults.getInt("x");
          final var y = tileResults.getInt("y");
          final var z = tileResults.getInt("z");
          final var data = tileResults.getBinaryStream("data").readAllBytes();

          config
              .taskRunner()
              .run(
                  () -> {
                    if (!convertTile(config, z, x, y, data, uniqueID, updateStatement)) {
                      success.set(false);
                    }
                  });
        }

        try {
          config.taskRunner().shutdown();
          config.taskRunner().awaitTermination();
        } catch (InterruptedException ex) {
          System.err.println("ERROR: Interrupted");
          return false;
        }
      }

      if (!config.continueOnError() && !success.get()) {
        return false;
      }

      updateMetadata(config, dstConnection);

      vacuumDatabase(dstConnection, config.verboseLevel());
    } catch (SQLException | IOException ex) {
      System.err.println("ERROR: Offline Database conversion failed: " + ex.getMessage());
      logErrorStack(ex, config.verboseLevel());
      return false;
    }
    return success.get();
  }

  private static void updateMetadata(@NonNull EncodeConfig config, Connection dstConnection)
      throws SQLException, IOException {
    var metadataKind = 2; // `mbgl::Resource::Kind::Source`
    var metadataQuerySQL = "SELECT id,data FROM resources WHERE kind = ?";
    var metadataUpdateSQL = "UPDATE resources SET data = ?, compressed = ? WHERE id = ?";
    try (var queryStatement = dstConnection.prepareStatement(metadataQuerySQL);
        var updateStatement = dstConnection.prepareStatement(metadataUpdateSQL)) {
      queryStatement.setInt(1, metadataKind);
      final var metadataResults = queryStatement.executeQuery();
      while (metadataResults.next()) {
        var uniqueID = metadataResults.getLong("id");
        byte[] data;
        try {
          data = decompress(metadataResults.getBinaryStream("data"));
        } catch (IOException | IllegalStateException ignore) {
          System.err.printf(
              "WARNING: Failed to decompress Source resource '%d', skipping%n", uniqueID);
          continue;
        }

        // Parse JSON
        var jsonString = new String(data, StandardCharsets.UTF_8);
        JsonObject json;
        try {
          json = new Gson().fromJson(jsonString, JsonObject.class);
        } catch (JsonSyntaxException ex) {
          System.err.printf("WARNING: Source resource '%d' is not JSON, skipping%n", uniqueID);
          continue;
        }

        // Update the format field
        json.addProperty("format", MBTilesHelper.MetadataMIMEType);

        // Re-serialize
        jsonString = json.toString();

        // Re-compress
        final boolean compressed;
        try (var outputStream = new ByteArrayOutputStream()) {
          try (var compressStream = createCompressStream(outputStream, config.compressionType())) {
            compressed = (compressStream != outputStream);
            compressStream.write(jsonString.getBytes(StandardCharsets.UTF_8));
          }
          data = outputStream.toByteArray();
        }

        // Update the database
        updateStatement.setBytes(1, data);
        updateStatement.setBoolean(2, compressed);
        updateStatement.setLong(3, uniqueID);
        updateStatement.execute();

        if (config.verboseLevel() > 1) {
          System.err.printf("Updated source JSON format to '%s'%n", MBTilesHelper.MetadataMIMEType);
        }
      }
    }
  }

  private static boolean convertTile(
      @NonNull EncodeConfig config,
      int z,
      int x,
      int y,
      byte[] data,
      long uniqueID,
      @NotNull PreparedStatement updateStatement) {
    if (config.verboseLevel() > 0) {
      System.err.printf("Converting %d:%d,%d%n", z, x, y);
    }

    byte[] srcTileData;
    try {
      srcTileData = decompress(new ByteArrayInputStream(data));
    } catch (IOException | IllegalStateException ex) {
      System.err.printf("ERROR: Failed to decompress tile '%d': %s%n", uniqueID, ex.getMessage());
      logErrorStack(ex, config.verboseLevel());
      return false;
    }

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
      try {
        // Parallel writes are possible, but only by creating a separate connection
        // for
        // each thread
        synchronized (updateStatement) {
          updateStatement.setBytes(1, tileData);
          updateStatement.setBoolean(2, didCompress.booleanValue());
          updateStatement.setLong(3, uniqueID);
          updateStatement.execute();
        }
        return true;
      } catch (SQLException ex) {
        System.err.printf("ERROR: Failed to convert tile '%d': %s%n", uniqueID, ex.getMessage());
        logErrorStack(ex, config.verboseLevel());
      }
    }
    return false;
  }
}
