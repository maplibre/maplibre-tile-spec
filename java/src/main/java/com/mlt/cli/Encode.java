package com.mlt.cli;

import com.google.gson.Gson;
import com.google.gson.JsonObject;
import com.google.gson.JsonSyntaxException;
import com.mlt.converter.ConversionConfig;
import com.mlt.converter.FeatureTableOptimizations;
import com.mlt.converter.MltConverter;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.data.MapLibreTile;
import com.mlt.decoder.MltDecoder;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.URI;
import java.net.URISyntaxException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.stream.Stream;
import java.util.zip.Inflater;
import java.util.zip.InflaterInputStream;
import javax.annotation.Nullable;
import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.HelpFormatter;
import org.apache.commons.cli.Option;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;
import org.apache.commons.compress.compressors.deflate.DeflateCompressorInputStream;
import org.apache.commons.compress.compressors.deflate.DeflateCompressorOutputStream;
import org.apache.commons.compress.compressors.deflate.DeflateParameters;
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream;
import org.apache.commons.compress.compressors.gzip.GzipCompressorOutputStream;
import org.apache.commons.compress.compressors.gzip.GzipParameters;
import org.apache.commons.io.EndianUtils;
import org.apache.commons.io.FilenameUtils;
import org.apache.commons.lang3.StringUtils;
import org.apache.commons.lang3.mutable.MutableBoolean;
import org.imintel.mbtiles4j.MBTilesReadException;
import org.imintel.mbtiles4j.MBTilesReader;
import org.imintel.mbtiles4j.MBTilesWriteException;
import org.imintel.mbtiles4j.MBTilesWriter;
import org.imintel.mbtiles4j.Tile;
import org.jetbrains.annotations.NotNull;

public class Encode {
  public static void main(String[] args) {
    try {
      var cmd = getCommandLine(args);
      if (cmd == null) {
        System.exit(1);
      }

      var tileFileName = cmd.getOptionValue(INPUT_TILE_ARG);
      var includeIds = !cmd.hasOption(EXCLUDE_IDS_OPTION);
      var useMortonEncoding = !cmd.hasOption(NO_MORTON_OPTION);
      var outlineFeatureTables = cmd.getOptionValues(OUTLINE_FEATURE_TABLES_OPTION);
      var useAdvancedEncodingSchemes = cmd.hasOption(ADVANCED_ENCODING_OPTION);
      var tessellateSource = cmd.getOptionValue(TESSELLATE_URL_OPTION, (String) null);
      var tessellateURI = (tessellateSource != null) ? new URI(tessellateSource) : null;
      var preTessellatePolygons =
          (tessellateSource != null) || cmd.hasOption(PRE_TESSELLATE_OPTION);
      var compressionType = cmd.getOptionValue(COMPRESS_OPTION, (String) null);
      var verbose = cmd.hasOption(VERBOSE_OPTION);

      // No ColumnMapping as support is still buggy:
      // https://github.com/maplibre/maplibre-tile-spec/issues/59
      var columnMappings = Optional.<List<ColumnMapping>>empty();

      var optimizations = new HashMap<String, FeatureTableOptimizations>();
      // TODO: Load layer -> optimizations map
      // each layer:
      //  new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);

      var conversionConfig =
          new ConversionConfig(
              includeIds,
              useAdvancedEncodingSchemes,
              optimizations,
              preTessellatePolygons,
              useMortonEncoding,
              (outlineFeatureTables != null ? List.of(outlineFeatureTables) : List.of()));

      if (cmd.hasOption(INPUT_TILE_ARG)) {
        // Converting one tile
        encodeTile(tileFileName, cmd, columnMappings, conversionConfig, tessellateURI, verbose);
      } else if (cmd.hasOption(INPUT_MBTILES_ARG)) {
        // Converting all the tiles in an MBTiles file
        var inputPath = cmd.getOptionValue(INPUT_MBTILES_ARG);
        var outputPath = getOutputPath(cmd, inputPath, "mlt.mbtiles");
        encodeMBTiles(
            inputPath,
            outputPath,
            columnMappings,
            conversionConfig,
            tessellateURI,
            compressionType,
            verbose);
      } else if (cmd.hasOption(INPUT_OFFLINEDB_ARG)) {
        var inputPath = cmd.getOptionValue(INPUT_OFFLINEDB_ARG);
        var ext = FilenameUtils.getExtension(inputPath);
        if (!ext.isEmpty()) {
          ext = "." + ext;
        }
        var outputPath = getOutputPath(cmd, inputPath, "mlt" + ext);
        encodeOfflineDB(
            Path.of(inputPath),
            outputPath,
            columnMappings,
            conversionConfig,
            tessellateURI,
            compressionType,
            verbose);
      }
    } catch (Exception e) {
      System.err.println("Failed:");
      e.printStackTrace(System.err);
      System.exit(1);
    }
  }

  ///  Convert a single tile from an individual file
  @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
  private static void encodeTile(
      String tileFileName,
      CommandLine cmd,
      Optional<List<ColumnMapping>> columnMappings,
      ConversionConfig conversionConfig,
      @Nullable URI tessellateSource,
      boolean verbose)
      throws IOException {
    var willOutput =
        cmd.hasOption(OUTPUT_FILE_ARG)
            || cmd.hasOption(OUTPUT_DIR_ARG)
            || cmd.hasOption(INCLUDE_EMBEDDED_METADATA)
            || cmd.hasOption(INCLUDE_EMBEDDED_PBF_METADATA);
    var willIncludeTilesetMetadata = cmd.hasOption(INCLUDE_TILESET_METADATA_OPTION);
    var willDecode = cmd.hasOption(DECODE_OPTION);
    var willPrintMLT = cmd.hasOption(PRINT_MLT_OPTION);
    var willPrintMVT = cmd.hasOption(PRINT_MVT_OPTION);
    var willCompare = cmd.hasOption(COMPARE_OPTION);
    var willUseVectorized = cmd.hasOption(VECTORIZED_OPTION);
    var willTime = cmd.hasOption(TIMER_OPTION);
    var inputTilePath = Paths.get(tileFileName);
    var inputTileName = inputTilePath.getFileName().toString();
    var outputPath = getOutputPath(cmd, inputTileName, "mlt");
    var decodedMvTile = MvtUtils.decodeMvt(inputTilePath);

    Timer timer = willTime ? new Timer() : null;

    var isIdPresent = true;
    var tileMetadata =
        MltConverter.createTilesetMetadata(List.of(decodedMvTile), columnMappings, isIdPresent);

    var mlTile =
        MltConverter.convertMvt(decodedMvTile, tileMetadata, conversionConfig, tessellateSource);
    if (willTime) {
      timer.stop("encoding");
    }

    if (willOutput) {
      if (outputPath != null) {
        if (verbose) {
          System.err.println("Writing converted tile to " + outputPath);
        }
        Files.write(outputPath, mlTile);

        if (willIncludeTilesetMetadata) {
          Path outputMetadataPath = Paths.get(outputPath.toString() + ".meta.pbf");
          if (verbose) {
            System.err.println("Writing metadata to " + outputMetadataPath);
          }
          tileMetadata.writeTo(Files.newOutputStream(outputMetadataPath));
        }
      }

      if (cmd.hasOption(INCLUDE_EMBEDDED_PBF_METADATA)) {
        var tileOutputPath = Paths.get(cmd.getOptionValue(INCLUDE_EMBEDDED_PBF_METADATA));
        writeTileWithEmbeddedMetadata(tileOutputPath, mlTile, tileMetadata);
      }
      if (cmd.hasOption(INCLUDE_EMBEDDED_METADATA)) {
        var tileOutputPath = Paths.get(cmd.getOptionValue(INCLUDE_EMBEDDED_METADATA));
        var metadata =
            MltConverter.createEmbeddedMetadata(
                List.of(decodedMvTile), columnMappings, isIdPresent);
        if (metadata != null) {
          writeTileWithEmbeddedMetadata(tileOutputPath, mlTile, metadata);
        } else {
          System.err.println("ERROR: Failed to generate embedded metadata");
        }
      }
    }
    if (willPrintMVT) {
      printMVT(decodedMvTile);
    }
    var needsDecoding = willDecode || willCompare || willPrintMLT;
    if (needsDecoding) {
      if (willTime) {
        timer.restart();
      }

      // convert MLT wire format to an MapLibreTile object
      if (willUseVectorized) {
        System.err.println("WARNING: Vectorized encoding is not available");
      }

      var decodedTile = MltDecoder.decodeMlTile(mlTile, tileMetadata);
      if (willTime) {
        timer.stop("decoding");
      }
      if (willPrintMLT) {
        CliUtil.printMLT(decodedTile);
      }
      if (willCompare) {
        compare(decodedTile, decodedMvTile);
      }
    }
  }

  /// Encode the entire contents of an MBTile file of MVT tiles
  @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
  private static void encodeMBTiles(
      String inputMBTilesPath,
      Path outputPath,
      Optional<List<ColumnMapping>> columnMappings,
      ConversionConfig conversionConfig,
      @Nullable URI tessellateSource,
      String compressionType,
      boolean verbose) {
    MBTilesReader mbTilesReader = null;
    try {
      mbTilesReader = new MBTilesReader(new File(inputMBTilesPath));

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
        var metadata = mbTilesReader.getMetadata();
        mbTilesWriter.addMetadataEntry(metadata);

        var tiles = mbTilesReader.getTiles();
        try {
          while (tiles.hasNext()) {
            convertTile(
                tiles.next(),
                mbTilesWriter,
                columnMappings,
                conversionConfig,
                tessellateSource,
                compressionType,
                verbose);
          }

          // mbtiles4j doesn't support types other than png and jpg,
          // so we have to set the format metadata the hard way.
          var dbFile = mbTilesWriter.close();
          var connectionString = "jdbc:sqlite:" + dbFile.getAbsolutePath();
          try (var connection = DriverManager.getConnection(connectionString)) {
            if (verbose) {
              System.err.printf("Setting tile MIME type to '%s'%n", embeddedPBFMetadataMIMEType);
            }
            var sql = "UPDATE metadata SET value = ? WHERE name = ?";
            try (var statement = connection.prepareStatement(sql)) {
              statement.setString(1, embeddedPBFMetadataMIMEType);
              statement.setString(2, "format");
              statement.execute();
            }

            if (verbose) {
              System.err.println("Optimizing database");
            }
            try (var statement = connection.prepareStatement("VACUUM")) {
              statement.execute();
            }
          }
        } finally {
          // mbtiles4j doesn't support `AutoCloseable`
          tiles.close();
        }
      } finally {
        mbTilesWriter.close();
      }
    } catch (MBTilesReadException | IOException | MBTilesWriteException | SQLException ex) {
      System.err.println("ERROR: MBTiles conversion failed: " + ex.getMessage());
      if (verbose) {
        ex.printStackTrace(System.err);
      }
    } finally {
      if (mbTilesReader != null) {
        mbTilesReader.close();
      }
    }
  }

  /// Encode the MVT tiles in an offline database file
  @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
  private static void encodeOfflineDB(
      Path inputPath,
      Path outputPath,
      Optional<List<ColumnMapping>> columnMappings,
      ConversionConfig conversionConfig,
      @Nullable URI tessellateSource,
      String compressionType,
      boolean verbose)
      throws ClassNotFoundException {
    // Start with a copy of the source file so we don't have to rebuild the complex schema
    if (verbose) {
      System.err.println("Creating target file");
    }
    try {
      if (outputPath == null) {
        var tempFile = File.createTempFile("encode-", "-db");
        outputPath = tempFile.toPath();
        tempFile.deleteOnExit();
      }
      Files.copy(
          inputPath,
          outputPath,
          StandardCopyOption.REPLACE_EXISTING,
          StandardCopyOption.COPY_ATTRIBUTES);
    } catch (IOException ex) {
      System.err.println("ERROR: Failed to create target file: " + ex.getMessage());
      return;
    }

    Class.forName("org.sqlite.JDBC");
    var srcConnectionString = "jdbc:sqlite:" + inputPath.toAbsolutePath();
    var dstConnectionString = "jdbc:sqlite:" + outputPath.toAbsolutePath();
    var updateSql = "UPDATE tiles SET data = ?, compressed = ? WHERE id = ?";
    try (var srcConnection = DriverManager.getConnection(srcConnectionString);
        var dstConnection = DriverManager.getConnection(dstConnectionString)) {
      // Convert Tiles
      try (var iterateStatement = srcConnection.createStatement();
          var tileResults = iterateStatement.executeQuery("SELECT * FROM tiles");
          var updateStatement = dstConnection.prepareStatement(updateSql)) {
        while (tileResults.next()) {
          var uniqueID = tileResults.getLong("id");
          var x = tileResults.getInt("x");
          var y = tileResults.getInt("y");
          var z = tileResults.getInt("z");

          byte[] srcTileData;
          try {
            srcTileData = decompress(tileResults.getBinaryStream("data"));
          } catch (IOException | IllegalStateException ignore) {
            System.err.printf("WARNING: Failed to decompress tile '%d', skipping%n", uniqueID);
            continue;
          }

          var didCompress = new MutableBoolean(false);
          var tileData =
              convertTile(
                  x,
                  y,
                  z,
                  srcTileData,
                  conversionConfig,
                  columnMappings,
                  tessellateSource,
                  compressionType,
                  didCompress,
                  verbose);

          if (tileData != null) {
            updateStatement.setBytes(1, tileData);
            updateStatement.setBoolean(2, didCompress.booleanValue());
            updateStatement.setLong(3, uniqueID);
            updateStatement.execute();

            if (verbose) {
              System.err.printf("Updated %d:%d,%d%n", z, x, y);
            }
          }
        }
      }

      // Update metadata
      var metadataKind = 2; // `mbgl::Resource::Kind::Source`
      var metadataQuerySQL = "SELECT id,data FROM resources WHERE kind = " + metadataKind;
      var metadataUpdateSQL = "UPDATE resources SET data = ?, compressed = ? WHERE id = ?";
      try (var queryStatement = dstConnection.createStatement();
          var metadataResults = queryStatement.executeQuery(metadataQuerySQL);
          var updateStatement = dstConnection.prepareStatement(metadataUpdateSQL)) {
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
          json.addProperty("format", embeddedPBFMetadataMIMEType);

          // Re-serialize
          jsonString = json.toString();

          // Re-compress
          try (var outputStream = new ByteArrayOutputStream()) {
            try (var compressStream = compressStream(outputStream, compressionType)) {
              compressStream.write(jsonString.getBytes(StandardCharsets.UTF_8));
            }
            outputStream.close();
            data = outputStream.toByteArray();
          }

          // Update the database
          updateStatement.setBytes(1, data);
          updateStatement.setBoolean(2, !StringUtils.isEmpty(compressionType));
          updateStatement.setLong(3, uniqueID);
          updateStatement.execute();

          if (verbose) {
            System.err.printf("Updated source JSON format to '%s'%n", embeddedPBFMetadataMIMEType);
          }
        }
      }

      if (verbose) {
        System.err.println("Optimizing database");
      }
      try (var statement = dstConnection.prepareStatement("VACUUM")) {
        statement.execute();
      }
    } catch (SQLException | IOException ex) {
      System.err.println("ERROR: Offline Database conversion failed: " + ex.getMessage());
      if (verbose) {
        ex.printStackTrace(System.err);
      }
    }
  }

  @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
  private static void convertTile(
      Tile tile,
      MBTilesWriter mbTilesWriter,
      Optional<List<ColumnMapping>> columnMappings,
      ConversionConfig conversionConfig,
      @Nullable URI tessellateSource,
      String compressionType,
      boolean verbose)
      throws IOException, MBTilesWriteException {
    var x = tile.getColumn();
    var y = tile.getRow();
    var z = tile.getZoom();

    var srcTileData = getTileData(tile);
    var didCompress = new MutableBoolean(false);
    var tileData =
        convertTile(
            x,
            y,
            z,
            srcTileData,
            conversionConfig,
            columnMappings,
            tessellateSource,
            compressionType,
            didCompress,
            verbose);

    if (tileData != null) {
      mbTilesWriter.addTile(tileData, z, x, y);
    }

    if (verbose) {
      System.err.printf("Added %d:%d,%d%n", z, x, y);
    }
  }

  @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
  @Nullable
  private static byte[] convertTile(
      int x,
      int y,
      int z,
      byte[] srcTileData,
      ConversionConfig conversionConfig,
      Optional<List<ColumnMapping>> columnMappings,
      URI tessellateSource,
      String compressionType,
      MutableBoolean didCompress,
      boolean verbose) {
    try {
      var decodedMvTile = MvtUtils.decodeMvt(srcTileData);

      var isIdPresent = true;
      var tileMetadata =
          MltConverter.createTilesetMetadata(List.of(decodedMvTile), columnMappings, isIdPresent);

      var mlTile =
          MltConverter.convertMvt(decodedMvTile, tileMetadata, conversionConfig, tessellateSource);

      byte[] tileData;
      try (var outputStream = new ByteArrayOutputStream()) {
        byte[] binaryMetadata;
        try (var metadataStream = new ByteArrayOutputStream()) {
          tileMetadata.writeTo(metadataStream);
          metadataStream.close();
          binaryMetadata = metadataStream.toByteArray();
        }
        if (binaryMetadata.length >= 1 << 16) {
          throw new NumberFormatException("Invalid metadata size");
        }

        try (var binStream = new DataOutputStream(outputStream)) {
          binStream.writeShort(EndianUtils.swapShort((short) binaryMetadata.length));
          binStream.flush();

          outputStream.write(binaryMetadata);
          outputStream.write(mlTile);
        }

        outputStream.close();
        tileData = outputStream.toByteArray();
      }

      if (compressionType != null) {
        try (var outputStream = new ByteArrayOutputStream()) {
          try (var compressStream = compressStream(outputStream, compressionType)) {
            compressStream.write(tileData);
          }
          outputStream.close();

          // If the compressed version is enough of an improvement to
          // justify the cost of decompressing it, use that instead.
          // Just guesses for now, these should be established with testing.
          final double ratioThreshold = 0.98;
          final int fixedThreshold = 20;
          if (outputStream.size() < tileData.length - fixedThreshold
              && outputStream.size() < tileData.length * ratioThreshold) {
            didCompress.setTrue();
            tileData = outputStream.toByteArray();
          } else {
            didCompress.setFalse();
          }
        }
      }
      return tileData;
    } catch (IOException ex) {
      System.err.printf(
          "ERROR: Failed to write tile with embedded PBF metadata (%d:%d,%d): %s%n",
          z, x, y, ex.getMessage());
      if (verbose) {
        ex.printStackTrace(System.err);
      }
    }
    return null;
  }

  private static OutputStream compressStream(OutputStream src, @NotNull String compressionType) {
    if (Objects.equals(compressionType, "gzip")) {
      try {
        var parameters = new GzipParameters();
        parameters.setCompressionLevel(9);
        return new GzipCompressorOutputStream(src, parameters);
      } catch (IOException ignore) {
      }
    }
    if (Objects.equals(compressionType, "deflate")) {
      var parameters = new DeflateParameters();
      parameters.setCompressionLevel(9);
      parameters.setWithZlibHeader(false);
      return new DeflateCompressorOutputStream(src);
    }
    return src;
  }

  private static byte[] getTileData(Tile tile) throws IOException {
    return decompress(tile.getData());
  }

  private static byte[] decompress(InputStream srcStream) throws IOException {
    try {
      InputStream decompressInputStream = null;
      if (srcStream.available() > 3) {
        var header = srcStream.readNBytes(4);
        srcStream.reset();

        if (DeflateCompressorInputStream.matches(header, header.length)) {
          // deflate with zlib header
          var inflater = new Inflater(/* nowrap= */ false);
          decompressInputStream = new InflaterInputStream(srcStream, inflater);
        } else if (header[0] == 0x1f && header[1] == (byte) 0x8b) {
          // TODO: why doesn't GZIPInputStream work here?
          // decompressInputStream = new GZIPInputStream(srcStream);
          decompressInputStream = new GzipCompressorInputStream(srcStream);
        }
      }

      if (decompressInputStream != null) {
        try (var outputStream = new ByteArrayOutputStream()) {
          decompressInputStream.transferTo(outputStream);
          outputStream.close();
          return outputStream.toByteArray();
        }
      }
    } catch (IndexOutOfBoundsException | IOException ex) {
      System.err.printf("Failed to decompress data: %s%n", ex.getMessage());
    }

    return srcStream.readAllBytes();
  }

  public static void printMVT(MapboxVectorTile mvTile) {
    var mvtLayers = mvTile.layers();
    for (var i = 0; i < mvtLayers.size(); i++) {
      var mvtLayer = mvtLayers.get(i);
      System.out.println(mvtLayer.name());
      var mvtFeatures = mvtLayer.features();
      for (var j = 0; j < mvtFeatures.size(); j++) {
        var mvtFeature = mvtFeatures.get(j);
        System.out.println("  " + mvtFeature);
      }
    }
  }

  public static void compare(MapLibreTile mlTile, MapboxVectorTile mvTile) {
    var mltLayers = mlTile.layers();
    var mvtLayers = mvTile.layers();
    if (mltLayers.size() != mvtLayers.size()) {
      throw new RuntimeException("Number of layers in MLT and MVT tiles do not match");
    }
    for (var i = 0; i < mvtLayers.size(); i++) {
      var mltLayer = mltLayers.get(i);
      var mvtLayer = mvtLayers.get(i);
      var mltFeatures = mltLayer.features();
      var mvtFeatures = mvtLayer.features();
      if (mltFeatures.size() != mvtFeatures.size()) {
        throw new RuntimeException("Number of features in MLT and MVT layers do not match");
      }
      for (var j = 0; j < mvtFeatures.size(); j++) {
        var mvtFeature = mvtFeatures.get(j);
        var mltFeature =
            mltFeatures.stream().filter(f -> f.id() == mvtFeature.id()).findFirst().orElse(null);
        if (mltFeature == null || mvtFeature.id() != mltFeature.id()) {
          throw new RuntimeException(
              "Feature IDs in MLT and MVT layers do not match: "
                  + mvtFeature.id()
                  + " != "
                  + ((mltFeature != null) ? mltFeature.id() : "(none)"));
        }
        var mltGeometry = mltFeature.geometry();
        var mvtGeometry = mvtFeature.geometry();
        if (!mltGeometry.equals(mvtGeometry)) {
          throw new RuntimeException(
              "Geometries in MLT and MVT layers do not match: "
                  + mvtGeometry
                  + " != "
                  + mltGeometry);
        }
        var mltProperties = mltFeature.properties();
        var mvtProperties = mvtFeature.properties();
        if (mvtProperties.size() != mltProperties.size()) {
          throw new RuntimeException("Number of properties in MLT and MVT features do not match");
        }
        var mvtPropertyKeys = mvtProperties.keySet();
        var mltPropertyKeys = mltProperties.keySet();
        // compare keys
        if (!mvtPropertyKeys.equals(mltPropertyKeys)) {
          throw new RuntimeException("Property keys in MLT and MVT features do not match");
        }
        // compare values
        var equalValues =
            mvtProperties.keySet().stream()
                .allMatch(key -> mvtProperties.get(key).equals(mltProperties.get(key)));
        if (!equalValues) {
          throw new RuntimeException(
              "Property values in MLT and MVT features do not match: \n'"
                  + mvtProperties
                  + "'\n'"
                  + mltProperties
                  + "'");
        }
      }
    }
  }

  /// Write the binary metadata before the tile data
  private static void writeTileWithEmbeddedMetadata(
      Path tileOutputPath, byte[] mlTile, byte[] tileMetadata) {
    try (var stream = Files.newOutputStream(tileOutputPath)) {
      stream.write(tileMetadata);
      stream.write(mlTile);
    } catch (IOException ex) {
      System.err.println("ERROR: Failed to write tile with embedded metadata");
      ex.printStackTrace(System.err);
    }
  }

  /// Write the serialized PBF metadata, preceded by a length header before the tile data
  private static void writeTileWithEmbeddedMetadata(
      Path tileOutputPath, byte[] mlTile, MltTilesetMetadata.TileSetMetadata tileMetadata) {
    try (var stream = Files.newOutputStream(tileOutputPath)) {
      byte[] binaryMetadata;
      try (var metadataStream = new ByteArrayOutputStream()) {
        tileMetadata.writeTo(metadataStream);
        metadataStream.flush();
        binaryMetadata = metadataStream.toByteArray();
      }
      try (var binStream = new DataOutputStream(stream)) {
        binStream.writeShort(binaryMetadata.length);
      }
      stream.write(binaryMetadata);
      stream.write(mlTile);
    } catch (IOException ex) {
      System.err.println("ERROR: Failed to write tile with embedded PBF metadata");
      ex.printStackTrace(System.err);
    }
  }

  private static final String INPUT_TILE_ARG = "mvt";
  private static final String INPUT_MBTILES_ARG = "mbtiles";
  private static final String INPUT_OFFLINEDB_ARG = "offlinedb";
  private static final String OUTPUT_DIR_ARG = "dir";
  private static final String OUTPUT_FILE_ARG = "mlt";
  private static final String EXCLUDE_IDS_OPTION = "noids";
  private static final String INCLUDE_TILESET_METADATA_OPTION = "metadata";
  private static final String INCLUDE_EMBEDDED_METADATA = "embedmetadata";
  private static final String INCLUDE_EMBEDDED_PBF_METADATA = "embedpbfmetadata";
  private static final String ADVANCED_ENCODING_OPTION = "advanced";
  private static final String NO_MORTON_OPTION = "nomorton";
  private static final String PRE_TESSELLATE_OPTION = "tessellate";
  private static final String TESSELLATE_URL_OPTION = "tessellateurl";
  private static final String OUTLINE_FEATURE_TABLES_OPTION = "outlines";
  private static final String DECODE_OPTION = "decode";
  private static final String PRINT_MLT_OPTION = "printmlt";
  private static final String PRINT_MVT_OPTION = "printmvt";
  private static final String COMPARE_OPTION = "compare";
  private static final String VECTORIZED_OPTION = "vectorized";
  private static final String TIMER_OPTION = "timer";
  private static final String COMPRESS_OPTION = "compress";
  private static final String COMPRESS_OPTION_DEFLATE = "deflate";
  private static final String COMPRESS_OPTION_GZIP = "gzip";
  private static final String COMPRESS_OPTION_NONE = "none";
  private static final String VERBOSE_OPTION = "verbose";
  private static final String HELP_OPTION = "help";

  /// Resolve an output filename.
  /// If an output filename is specified directly, use it.
  /// If only an output directory is given, add the input filename and the specified extension.
  /// If neither a directory or file name is given, returns null.  This is used for testing.
  /// If a path is returned and the directory doesn't already exist, it is created.
  private static @Nullable Path getOutputPath(
      CommandLine cmd, String inputFileName, String targetExt) {
    Path outputPath = null;
    if (cmd.hasOption(OUTPUT_DIR_ARG)) {
      var outputDir = cmd.getOptionValue(OUTPUT_DIR_ARG);
      var baseName = FilenameUtils.getBaseName(inputFileName);
      outputPath = Paths.get(outputDir, baseName + "." + targetExt);
    } else if (cmd.hasOption(OUTPUT_FILE_ARG)) {
      outputPath = Paths.get(cmd.getOptionValue(OUTPUT_FILE_ARG));
    }
    if (outputPath != null) {
      var outputDirPath = outputPath.toAbsolutePath().getParent();
      if (!Files.exists(outputDirPath)) {
        try {
          Files.createDirectories(outputDirPath);
          System.err.println("Created directory: " + outputDirPath);
        } catch (IOException ex) {
          System.err.println("Failed to create directory: " + outputDirPath);
          ex.printStackTrace(System.err);
        }
      }
    }
    return outputPath;
  }

  private static CommandLine getCommandLine(String[] args) throws ParseException {
    try {
      Options options = new Options();
      options.addOption(
          Option.builder()
              .longOpt(INPUT_TILE_ARG)
              .hasArg(true)
              .argName("file")
              .desc("Path to the input MVT file")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(INPUT_MBTILES_ARG)
              .hasArg(true)
              .argName("file")
              .desc("Path of the input MBTiles file.")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(INPUT_OFFLINEDB_ARG)
              .hasArg(true)
              .argName("file")
              .desc("Path of the input offline database file.")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(OUTPUT_DIR_ARG)
              .hasArg(true)
              .argName("dir")
              .desc(
                  "Directory where the output is written, using the input file basename (OPTIONAL).")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(OUTPUT_FILE_ARG)
              .hasArg(true)
              .argName("file")
              .desc(
                  "Filename where the output will be written. Overrides --" + OUTPUT_DIR_ARG + ".")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(EXCLUDE_IDS_OPTION)
              .hasArg(false)
              .desc("Don't include feature IDs.")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(INCLUDE_TILESET_METADATA_OPTION)
              .hasArg(false)
              .desc(
                  "Write tileset metadata (PBF) alongside the output tile (adding '.meta.pbf'). "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(INCLUDE_EMBEDDED_METADATA)
              .hasArg(true)
              .argName("file")
              .desc(
                  "Write output tile with embedded metadata. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(INCLUDE_EMBEDDED_PBF_METADATA)
              .hasArg(true)
              .argName("file")
              .desc(
                  "Write output with embedded PBF metadata"
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(ADVANCED_ENCODING_OPTION)
              .hasArg(false)
              .desc("Enable advanced encodings (FSST & FastPFOR).")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(NO_MORTON_OPTION)
              .hasArg(false)
              .desc("Disable Morton encoding.")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(PRE_TESSELLATE_OPTION)
              .hasArg(false)
              .desc("Include tessellation data in converted tiles.")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(TESSELLATE_URL_OPTION)
              .hasArg(true)
              .desc("Use a tessellation server (implies --" + PRE_TESSELLATE_OPTION + ").")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(OUTLINE_FEATURE_TABLES_OPTION)
              .hasArgs()
              .desc(
                  "The feature tables for which outlines are included "
                      + "([OPTIONAL], comma-separated, * for all, default: none).")
              .valueSeparator(',')
              .argName("tables")
              .required(false)
              .optionalArg(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(DECODE_OPTION)
              .hasArg(false)
              .desc(
                  "Test decoding the tile after encoding it. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(PRINT_MLT_OPTION)
              .hasArg(false)
              .desc(
                  "Print the MLT tile after encoding it. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(PRINT_MVT_OPTION)
              .hasArg(false)
              .desc(
                  "Print the round-tripped MVT tile. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(COMPARE_OPTION)
              .hasArg(false)
              .desc(
                  "Assert that data in the the decoded tile is the same as the input tile. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(VECTORIZED_OPTION)
              .hasArg(false)
              .desc("Use the vectorized decoding path.")
              .required(false)
              .deprecated()
              .build());
      options.addOption(
          Option.builder()
              .longOpt(TIMER_OPTION)
              .hasArg(false)
              .desc("Print the time it takes, in ms, to decode a tile.")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(COMPRESS_OPTION)
              .hasArg(true)
              .argName("algorithm")
              .desc(
                  "Compress tile data with one of 'deflate', 'gzip'. "
                      + "Only applies to MBTiles and offline databases.")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .option("v")
              .longOpt(VERBOSE_OPTION)
              .hasArg(false)
              .desc("Enable verbose output.")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .option("h")
              .longOpt(HELP_OPTION)
              .hasArg(false)
              .desc("Show this output.")
              .required(false)
              .build());

      var cmd = new DefaultParser().parse(options, args);

      var tessellateSource = cmd.getOptionValue(TESSELLATE_URL_OPTION, (String) null);
      if (tessellateSource != null) {
        new URI(tessellateSource);
      }

      if (cmd.getOptions().length == 0 || cmd.hasOption(HELP_OPTION)) {
        var width = 100;
        var autoUsage = true;
        var header =
            "\nConvert an MVT tile file or MBTiles containing MVT tiles to MLT format.\n\n";
        var footer = "";
        var formatter = new HelpFormatter();
        formatter.setOptionComparator(null);
        formatter.printHelp(width, Encode.class.getName(), header, options, footer, autoUsage);
      } else if (Stream.of(
                  cmd.hasOption(INPUT_TILE_ARG),
                  cmd.hasOption(INPUT_MBTILES_ARG),
                  cmd.hasOption(INPUT_OFFLINEDB_ARG))
              .filter(x -> x)
              .count()
          != 1) {
        System.err.println(
            "Specify one of '--"
                + INPUT_TILE_ARG
                + "', '--"
                + INPUT_MBTILES_ARG
                + "', and '--"
                + INPUT_OFFLINEDB_ARG
                + "'.");
      } else if (cmd.hasOption(OUTPUT_FILE_ARG) && cmd.hasOption(OUTPUT_DIR_ARG)) {
        System.err.println(
            "Cannot specify both '-"
                + OUTPUT_FILE_ARG
                + "' and '-"
                + OUTPUT_DIR_ARG
                + "' options.");
      } else if (!new HashSet<>(
              Arrays.asList(COMPRESS_OPTION_GZIP, COMPRESS_OPTION_DEFLATE, COMPRESS_OPTION_NONE))
          .contains(cmd.getOptionValue(COMPRESS_OPTION, COMPRESS_OPTION_NONE))) {
        System.err.println("Invalid compression type.");
      } else {
        return cmd;
      }
    } catch (ParseException ex) {
      System.err.println("Failed to parse options: " + ex.getMessage());
    } catch (URISyntaxException e) {
      System.err.println("Invalid tessellation URL: " + e.getMessage());
    }
    return null;
  }

  private static final String embeddedPBFMetadataMIMEType = "application/vnd.maplibre-tile-pbf";
}
