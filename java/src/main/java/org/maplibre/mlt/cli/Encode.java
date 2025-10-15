package org.maplibre.mlt.cli;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonObject;
import com.google.gson.JsonSyntaxException;
import jakarta.annotation.Nullable;
import java.io.ByteArrayOutputStream;
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
import java.util.Map;
import java.util.Objects;
import java.util.TreeMap;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import java.util.zip.Inflater;
import java.util.zip.InflaterInputStream;
import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Option;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;
import org.apache.commons.cli.help.HelpFormatter;
import org.apache.commons.compress.compressors.deflate.DeflateCompressorInputStream;
import org.apache.commons.compress.compressors.deflate.DeflateCompressorOutputStream;
import org.apache.commons.compress.compressors.deflate.DeflateParameters;
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream;
import org.apache.commons.compress.compressors.gzip.GzipCompressorOutputStream;
import org.apache.commons.compress.compressors.gzip.GzipParameters;
import org.apache.commons.io.FilenameUtils;
import org.apache.commons.lang3.StringUtils;
import org.apache.commons.lang3.mutable.MutableBoolean;
import org.imintel.mbtiles4j.MBTilesReadException;
import org.imintel.mbtiles4j.MBTilesReader;
import org.imintel.mbtiles4j.MBTilesWriteException;
import org.imintel.mbtiles4j.MBTilesWriter;
import org.imintel.mbtiles4j.Tile;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.FeatureTableOptimizations;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.converter.MLTStreamObserverDefault;
import org.maplibre.mlt.converter.MLTStreamObserverFile;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.decoder.MltDecoder;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class Encode {
  public static void main(String[] args) {
    try {
      var cmd = getCommandLine(args);
      if (cmd == null) {
        System.exit(1);
      }

      final var tileFileName = cmd.getOptionValue(INPUT_TILE_ARG);
      final var includeIds = !cmd.hasOption(EXCLUDE_IDS_OPTION);
      final var useMortonEncoding = !cmd.hasOption(NO_MORTON_OPTION);
      final var outlineFeatureTables = cmd.getOptionValues(OUTLINE_FEATURE_TABLES_OPTION);
      final var useAdvancedEncodingSchemes = cmd.hasOption(ADVANCED_ENCODING_OPTION);
      final var tessellateSource = cmd.getOptionValue(TESSELLATE_URL_OPTION, (String) null);
      final var tessellateURI = (tessellateSource != null) ? new URI(tessellateSource) : null;
      final var tessellatePolygons =
          (tessellateSource != null) || cmd.hasOption(PRE_TESSELLATE_OPTION);
      final var compressionType = cmd.getOptionValue(COMPRESS_OPTION, (String) null);
      final var enableCoerceOnTypeMismatch = cmd.hasOption(ALLOW_COERCE_OPTION);
      final var enableElideOnTypeMismatch = cmd.hasOption(ALLOW_ELISION_OPTION);
      final var verbose = cmd.hasOption(VERBOSE_OPTION);
      final var filterRegex = cmd.getOptionValue(FILTER_LAYERS_OPTION, (String) null);
      final var filterPattern = (filterRegex != null) ? Pattern.compile(filterRegex) : null;
      final var filterInvert = cmd.hasOption(FILTER_LAYERS_INVERT_OPTION);

      // No ColumnMapping as support is still buggy:
      // https://github.com/maplibre/maplibre-tile-spec/issues/59
      final List<ColumnMapping> columnMappings = List.of();

      var optimizations = new HashMap<String, FeatureTableOptimizations>();
      // TODO: Load layer -> optimizations map
      // each layer:
      //  new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);

      var conversionConfig =
          new ConversionConfig(
              includeIds,
              useAdvancedEncodingSchemes,
              enableCoerceOnTypeMismatch,
              optimizations,
              tessellatePolygons,
              useMortonEncoding,
              (outlineFeatureTables != null ? List.of(outlineFeatureTables) : List.of()),
              filterPattern,
              filterInvert);

      if (verbose && outlineFeatureTables != null && outlineFeatureTables.length > 0) {
        System.err.println(
            "Including outlines for layers: " + String.join(", ", outlineFeatureTables));
      }

      if (cmd.hasOption(INPUT_TILE_ARG)) {
        // Converting one tile
        encodeTile(
            tileFileName,
            cmd,
            columnMappings,
            conversionConfig,
            tessellateURI,
            enableElideOnTypeMismatch,
            verbose);
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
            enableElideOnTypeMismatch,
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
            enableElideOnTypeMismatch,
            verbose);
      }
      if (verbose && totalCompressedInput > 0) {
        System.err.printf(
            "Compressed %d bytes to %d bytes (%.1f%%)%n",
            totalCompressedInput,
            totalCompressedOutput,
            100 * (double) totalCompressedOutput / totalCompressedInput);
      }
    } catch (Exception e) {
      System.err.println("Failed:");
      e.printStackTrace(System.err);
      System.exit(1);
    }
  }

  ///  Convert a single tile from an individual file
  private static void encodeTile(
      String tileFileName,
      CommandLine cmd,
      List<ColumnMapping> columnMappings,
      ConversionConfig conversionConfig,
      @Nullable URI tessellateSource,
      boolean enableElideOnTypeMismatch,
      boolean verbose)
      throws IOException {
    final var willOutput = cmd.hasOption(OUTPUT_FILE_ARG) || cmd.hasOption(OUTPUT_DIR_ARG);
    final var willDecode = cmd.hasOption(DECODE_OPTION);
    final var willPrintMLT = cmd.hasOption(PRINT_MLT_OPTION);
    final var willPrintMVT = cmd.hasOption(PRINT_MVT_OPTION);
    final var compareProp = cmd.hasOption(COMPARE_PROP_OPTION) || cmd.hasOption(COMPARE_ALL_OPTION);
    final var compareGeom = cmd.hasOption(COMPARE_GEOM_OPTION) || cmd.hasOption(COMPARE_ALL_OPTION);
    final var willCompare = compareProp || compareGeom;
    final var willUseVectorized = cmd.hasOption(VECTORIZED_OPTION);
    final var willTime = cmd.hasOption(TIMER_OPTION);
    final var inputTilePath = Paths.get(tileFileName);
    final var inputTileName = inputTilePath.getFileName().toString();
    final var outputPath = getOutputPath(cmd, inputTileName, "mlt");
    final var decodedMvTile = MvtUtils.decodeMvt(inputTilePath);

    final Timer timer = willTime ? new Timer() : null;

    final var isIdPresent = true;
    final var metadata =
        MltConverter.createTilesetMetadata(
            decodedMvTile,
            columnMappings,
            isIdPresent,
            conversionConfig.getCoercePropertyValues(),
            enableElideOnTypeMismatch);
    final var metadataJSON = MltConverter.createTilesetMetadataJSON(metadata);

    MLTStreamObserver streamObserver = new MLTStreamObserverDefault();
    if (cmd.hasOption(DUMP_STREAMS_OPTION)) {
      final var fileName = MLTStreamObserverFile.sanitizeFilename(inputTileName);
      final var streamPath = getOutputPath(cmd, fileName, null, true);
      if (streamPath != null) {
        streamObserver = new MLTStreamObserverFile(streamPath);
        Files.createDirectories(streamPath);
        if (verbose) {
          System.err.println("Writing raw streams to " + streamPath);
        }
      }
    }
    var mlTile =
        MltConverter.convertMvt(
            decodedMvTile, metadata, conversionConfig, tessellateSource, streamObserver);
    if (willTime) {
      timer.stop("encoding");
    }

    if (willOutput) {
      if (outputPath != null) {
        if (verbose) {
          System.err.println("Writing converted tile to " + outputPath);
        }

        try {
          Files.write(outputPath, mlTile);
        } catch (IOException ex) {
          System.err.println("ERROR: Failed to write tile");
          ex.printStackTrace(System.err);
        }

        if (cmd.hasOption(INCLUDE_TILESET_METADATA_OPTION)) {
          var metadataPath = outputPath.resolveSibling(outputPath.getFileName() + ".json");
          if (verbose) {
            System.err.println("Writing tileset metadata to " + metadataPath);
          }
          Files.writeString(metadataPath, metadataJSON);
        }
      }
    }
    if (willPrintMVT) {
      System.out.write(printMVT(decodedMvTile).getBytes(StandardCharsets.UTF_8));
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

      var decodedTile = MltDecoder.decodeMlTile(mlTile);
      if (willTime) {
        timer.stop("decoding");
      }
      if (willPrintMLT) {
        System.out.write(CliUtil.printMLT(decodedTile).getBytes(StandardCharsets.UTF_8));
      }
      if (willCompare) {
        compare(decodedTile, decodedMvTile, compareGeom, compareProp);
      }
    }
  }

  /// Encode the entire contents of an MBTile file of MVT tiles
  private static void encodeMBTiles(
      String inputMBTilesPath,
      Path outputPath,
      List<ColumnMapping> columnMappings,
      ConversionConfig conversionConfig,
      @Nullable URI tessellateSource,
      String compressionType,
      boolean enableCoerceOrElideMismatch,
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

        var pbMeta = MltTilesetMetadata.TileSetMetadata.newBuilder();
        pbMeta.setName(metadata.getTilesetName());
        pbMeta.setAttribution(metadata.getAttribution());
        pbMeta.setDescription(metadata.getTilesetDescription());
        var bounds = metadata.getTilesetBounds();
        pbMeta.addAllBounds(
            List.of(bounds.getLeft(), bounds.getTop(), bounds.getRight(), bounds.getBottom()));
        var metadataJSON = MltConverter.createTilesetMetadataJSON(pbMeta.build());

        var tiles = mbTilesReader.getTiles();
        try {
          while (tiles.hasNext()) {
            final var tile = tiles.next();
            try {
              convertTile(
                  tile,
                  mbTilesWriter,
                  columnMappings,
                  conversionConfig,
                  tessellateSource,
                  compressionType,
                  enableCoerceOrElideMismatch,
                  verbose);
            } catch (IllegalArgumentException ex) {
              System.err.printf(
                  "WARNING: Failed to convert tile (%d:%d,%d) : %s%n",
                  tile.getZoom(), tile.getColumn(), tile.getRow(), ex.getMessage());
              if (verbose) {
                ex.printStackTrace(System.err);
              }
            }
          }

          // mbtiles4j doesn't support types other than png and jpg,
          // so we have to set the format metadata the hard way.
          var dbFile = mbTilesWriter.close();
          var connectionString = "jdbc:sqlite:" + dbFile.getAbsolutePath();
          try (var connection = DriverManager.getConnection(connectionString)) {
            if (verbose) {
              System.err.printf("Setting tile MIME type to '%s'%n", MetadataMIMEType);
            }
            var sql = "UPDATE metadata SET value = ? WHERE name = ?";
            try (var statement = connection.prepareStatement(sql)) {
              statement.setString(1, MetadataMIMEType);
              statement.setString(2, "format");
              statement.execute();
            }

            // Put the global metadata in a custom metadata key.
            // Could also be in a custom key within the standard `json` entry...
            if (verbose) {
              System.err.println("Adding tileset metadata JSON");
            }
            sql = "INSERT OR REPLACE INTO metadata (name, value) VALUES (?, ?)";
            try (var statement = connection.prepareStatement(sql)) {
              statement.setString(1, "mln-json");
              statement.setString(2, metadataJSON);
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
  private static void encodeOfflineDB(
      Path inputPath,
      Path outputPath,
      List<ColumnMapping> columnMappings,
      ConversionConfig conversionConfig,
      @Nullable URI tessellateSource,
      String compressionType,
      boolean enableCoerceOrElideMismatch,
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
                  enableCoerceOrElideMismatch,
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
          json.addProperty("format", MetadataMIMEType);

          // Re-serialize
          jsonString = json.toString();

          // Re-compress
          try (var outputStream = new ByteArrayOutputStream()) {
            try (var compressStream = compressStream(outputStream, compressionType)) {
              compressStream.write(jsonString.getBytes(StandardCharsets.UTF_8));
            }
            data = outputStream.toByteArray();
          }

          // Update the database
          updateStatement.setBytes(1, data);
          updateStatement.setBoolean(2, !StringUtils.isEmpty(compressionType));
          updateStatement.setLong(3, uniqueID);
          updateStatement.execute();

          if (verbose) {
            System.err.printf("Updated source JSON format to '%s'%n", MetadataMIMEType);
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

  private static void convertTile(
      Tile tile,
      MBTilesWriter mbTilesWriter,
      List<ColumnMapping> columnMappings,
      ConversionConfig conversionConfig,
      @Nullable URI tessellateSource,
      String compressionType,
      boolean enableCoerceOrElideMismatch,
      boolean verbose)
      throws IOException, MBTilesWriteException {
    final var x = tile.getColumn();
    final var y = tile.getRow();
    final var z = tile.getZoom();

    if (verbose) {
      System.err.printf("Converting %d:%d,%d%n", z, x, y);
    }

    final var srcTileData = getTileData(tile);
    final var didCompress = new MutableBoolean(false);
    final var tileData =
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
            enableCoerceOrElideMismatch,
            verbose);

    if (tileData != null) {
      mbTilesWriter.addTile(tileData, z, x, y);
    }
  }

  @Nullable
  private static byte[] convertTile(
      int x,
      int y,
      int z,
      byte[] srcTileData,
      ConversionConfig conversionConfig,
      List<ColumnMapping> columnMappings,
      URI tessellateSource,
      String compressionType,
      MutableBoolean didCompress,
      boolean enableElideOnMismatch,
      boolean verbose) {
    try {
      final var decodedMvTile = MvtUtils.decodeMvt(srcTileData);

      final var isIdPresent = true;
      final var coercePropertyValues = conversionConfig.getCoercePropertyValues();
      final var metadata =
          MltConverter.createTilesetMetadata(
              decodedMvTile,
              columnMappings,
              isIdPresent,
              coercePropertyValues,
              enableElideOnMismatch);
      var tileData =
          MltConverter.convertMvt(decodedMvTile, metadata, conversionConfig, tessellateSource);

      if (compressionType != null) {
        try (var outputStream = new ByteArrayOutputStream()) {
          try (var compressStream = compressStream(outputStream, compressionType)) {
            compressStream.write(tileData);
          }

          // If the compressed version is enough of an improvement to
          // justify the cost of decompressing it, use that instead.
          // Just guesses for now, these should be established with testing and/or configurable.
          final double ratioThreshold = 0.98;
          final int fixedThreshold = 20;
          if (outputStream.size() < tileData.length - fixedThreshold
              && outputStream.size() < tileData.length * ratioThreshold) {
            if (verbose) {
              System.err.printf(
                  "Compressed %d:%d,%d from %d to %d bytes (%.1f%%)%n",
                  z,
                  x,
                  y,
                  tileData.length,
                  outputStream.size(),
                  100 * (double) outputStream.size() / tileData.length);
              totalCompressedInput += tileData.length;
              totalCompressedOutput += outputStream.size();
            }
            didCompress.setTrue();
            tileData = outputStream.toByteArray();
          } else {
            if (verbose) {
              System.err.printf(
                  "Compression of %d:%d,%d not effective, saving uncompressed (%d vs %d bytes)%n",
                  z, x, y, tileData.length, outputStream.size());
            }
            didCompress.setFalse();
          }
        }
      }
      return tileData;
    } catch (IOException ex) {
      System.err.printf("ERROR: Failed to write tile (%d:%d,%d): %s%n", z, x, y, ex.getMessage());
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
          return outputStream.toByteArray();
        }
      }
    } catch (IndexOutOfBoundsException | IOException ex) {
      System.err.printf("Failed to decompress data: %s%n", ex.getMessage());
    }

    return srcStream.readAllBytes();
  }

  public static String printMVT(MapboxVectorTile mvTile) {
    final var gson = new GsonBuilder().setPrettyPrinting().create();
    return gson.toJson(Map.of("layers", mvTile.layers().stream().map(Encode::toJSON).toList()));
  }

  private static Map<String, Object> toJSON(Layer layer) {
    var map = new TreeMap<String, Object>();
    map.put("name", layer.name());
    map.put("extent", layer.tileExtent());
    map.put("features", layer.features().stream().map(Encode::toJSON).toList());
    return map;
  }

  private static Map<String, Object> toJSON(Feature feature) {
    var map = new TreeMap<String, Object>();
    map.put("id", feature.id());
    map.put("geometry", feature.geometry().toString());
    // Print properties sorted by key to allow for direct comparison with MLT output.
    map.put(
        "properties",
        feature.properties().entrySet().stream()
            .filter(entry -> entry.getValue() != null)
            .collect(
                Collectors.toMap(
                    Map.Entry::getKey, Map.Entry::getValue, (v1, v2) -> v1, TreeMap::new)));
    return map;
  }

  private static boolean propertyValuesEqual(Object a, Object b) {
    // Try simple equality
    if (Objects.equals(a, b)) {
      return true;
    }
    // Allow for, e.g., int32 and int64 representations of the same number by comparing strings
    return a.toString().equals(b.toString());
  }

  public static void compare(
      MapLibreTile mlTile, MapboxVectorTile mvTile, boolean compareGeom, boolean compareProp) {
    final var mltLayers = mlTile.layers();
    final var mvtLayers = mvTile.layers().stream().filter(x -> !x.features().isEmpty()).toList();
    if (mltLayers.size() != mvtLayers.size()) {
      final var mvtNames = mvtLayers.stream().map(Layer::name).collect(Collectors.joining(", "));
      final var mltNames = mltLayers.stream().map(Layer::name).collect(Collectors.joining(", "));
      throw new RuntimeException(
          "Number of layers in MLT and MVT tiles do not match:\nMVT:\n"
              + mvtNames
              + "\nMLT:\n"
              + mltNames);
    }
    for (var i = 0; i < mvtLayers.size(); i++) {
      final var mltLayer = mltLayers.get(i);
      final var mvtLayer = mvtLayers.get(i);
      final var mltFeatures = mltLayer.features();
      final var mvtFeatures = mvtLayer.features();
      if (!mltLayer.name().equals(mvtLayer.name())) {
        throw new RuntimeException(
            "Layer index "
                + i
                + " of MVT and MLT tile differ: '"
                + mvtLayer.name()
                + "' != '"
                + mltLayer.name()
                + "'");
      }
      if (mltFeatures.size() != mvtFeatures.size()) {
        throw new RuntimeException(
            "Number of features in MLT and MVT layer '"
                + mvtLayer.name()
                + "' do not match: "
                + mltFeatures.size()
                + " != "
                + mvtFeatures.size());
      }
      for (var j = 0; j < mvtFeatures.size(); j++) {
        final var mvtFeature = mvtFeatures.get(j);
        // Expect features to be written in the same order
        final var mltFeature = mltFeatures.get(j);
        if (mvtFeature.id() != mltFeature.id()) {
          throw new RuntimeException(
              "Feature IDs for index "
                  + j
                  + " in MLT and MVT layers do not match: "
                  + mvtFeature.id()
                  + " != "
                  + mltFeature.id());
        }
        if (compareGeom) {
          final var mltGeometry = mltFeature.geometry();
          final var mltGeomValid = mltGeometry.isValid();
          final var mvtGeometry = mvtFeature.geometry();
          final var mvtGeomValid = mvtGeometry.isValid();
          if (mltGeomValid != mvtGeomValid) {
            throw new RuntimeException(
                "Geometry validity in MLT and MVT layers do not match: \nMVT:\n"
                    + mvtGeomValid
                    + "\nMLT:\n"
                    + mltGeomValid);
          }

          if (mvtGeomValid && !mltGeometry.equals(mvtGeometry)) {
            throw new RuntimeException(
                "Geometries in MLT and MVT layers do not match: \nMVT:\n"
                    + mvtGeometry
                    + "\nMLT:\n"
                    + mltGeometry
                    + "\nDifference:\n"
                    + mvtGeometry.difference(mltGeometry));
          }
        }
        if (compareProp) {
          final var mltProperties = mltFeature.properties();
          final var mvtProperties = mvtFeature.properties();
          final var mvtPropertyKeys = mvtProperties.keySet();
          final var nonNullMLTKeys =
              mltProperties.entrySet().stream()
                  .filter(entry -> entry.getValue() != null)
                  .map(Map.Entry::getKey)
                  .collect(Collectors.toUnmodifiableSet());
          // compare keys
          if (!mvtPropertyKeys.equals(nonNullMLTKeys)) {
            throw new RuntimeException(
                "Property keys in MLT and MVT feature index "
                    + j
                    + " in layer '"
                    + mvtLayer.name()
                    + "' do not match:\nMVT:\n"
                    + String.join(",", mvtPropertyKeys)
                    + "\nMLT:\n"
                    + String.join(",", nonNullMLTKeys));
          }
          // compare values
          final var unequalKeys =
              mvtProperties.keySet().stream()
                  .filter(
                      key -> !propertyValuesEqual(mvtProperties.get(key), mltProperties.get(key)))
                  .toList();
          if (!unequalKeys.isEmpty()) {
            final var unequalValues =
                unequalKeys.stream()
                    .map(
                        key ->
                            "  "
                                + key
                                + ": mvt="
                                + mvtProperties.get(key)
                                + ", mlt="
                                + mltProperties.get(key)
                                + "\n")
                    .collect(Collectors.joining());
            throw new RuntimeException(
                "Property values in MLT and MVT feature index "
                    + j
                    + " in layer '"
                    + mvtLayer.name()
                    + "' do not match: \n"
                    + unequalValues);
          }
        }
      }
    }
  }

  private static final String INPUT_TILE_ARG = "mvt";
  private static final String INPUT_MBTILES_ARG = "mbtiles";
  private static final String INPUT_OFFLINEDB_ARG = "offlinedb";
  private static final String OUTPUT_DIR_ARG = "dir";
  private static final String OUTPUT_FILE_ARG = "mlt";
  private static final String EXCLUDE_IDS_OPTION = "noids";
  private static final String FILTER_LAYERS_OPTION = "filter-layers";
  private static final String FILTER_LAYERS_INVERT_OPTION = "filter-layers-invert";
  private static final String INCLUDE_METADATA_OPTION = "metadata";
  private static final String INCLUDE_TILESET_METADATA_OPTION = "tilesetmetadata";
  private static final String ALLOW_COERCE_OPTION = "coerce-mismatch";
  private static final String ALLOW_ELISION_OPTION = "elide-mismatch";
  private static final String ADVANCED_ENCODING_OPTION = "advanced";
  private static final String NO_MORTON_OPTION = "nomorton";
  private static final String PRE_TESSELLATE_OPTION = "tessellate";
  private static final String TESSELLATE_URL_OPTION = "tessellateurl";
  private static final String OUTLINE_FEATURE_TABLES_OPTION = "outlines";
  private static final String DECODE_OPTION = "decode";
  private static final String PRINT_MLT_OPTION = "printmlt";
  private static final String PRINT_MVT_OPTION = "printmvt";
  private static final String COMPARE_ALL_OPTION = "compare-all";
  private static final String COMPARE_GEOM_OPTION = "compare-geometry";
  private static final String COMPARE_PROP_OPTION = "compare-properties";
  private static final String VECTORIZED_OPTION = "vectorized";
  private static final String TIMER_OPTION = "timer";
  private static final String COMPRESS_OPTION = "compress";
  private static final String COMPRESS_OPTION_DEFLATE = "deflate";
  private static final String COMPRESS_OPTION_GZIP = "gzip";
  private static final String COMPRESS_OPTION_NONE = "none";
  private static final String DUMP_STREAMS_OPTION = "rawstreams";
  private static final String VERBOSE_OPTION = "verbose";
  private static final String HELP_OPTION = "help";

  /// Resolve an output filename.
  /// If an output filename is specified directly, use it.
  /// If only an output directory is given, add the input filename and the specified extension.
  /// If neither a directory or file name is given, returns null.  This is used for testing.
  /// If a path is returned and the directory doesn't already exist, it is created.
  private static @Nullable Path getOutputPath(
      CommandLine cmd, String inputFileName, String targetExt) {
    return getOutputPath(cmd, inputFileName, targetExt, false);
  }

  private static @Nullable Path getOutputPath(
      CommandLine cmd, String inputFileName, String targetExt, boolean forceExt) {
    final var ext =
        (targetExt != null && !targetExt.isEmpty())
            ? FilenameUtils.EXTENSION_SEPARATOR_STR + targetExt
            : "";
    Path outputPath = null;
    if (cmd.hasOption(OUTPUT_DIR_ARG)) {
      var outputDir = cmd.getOptionValue(OUTPUT_DIR_ARG);
      var baseName = FilenameUtils.getBaseName(inputFileName);
      outputPath = Paths.get(outputDir, baseName + ext);
    } else if (cmd.hasOption(OUTPUT_FILE_ARG)) {
      outputPath = Paths.get(cmd.getOptionValue(OUTPUT_FILE_ARG));
    }
    if (outputPath != null) {
      if (forceExt) {
        outputPath = Path.of(FilenameUtils.removeExtension(outputPath.toString()) + ext);
      }
      createDir(outputPath.toAbsolutePath().getParent());
    }
    return outputPath;
  }

  private static void createDir(Path path) {
    if (!Files.exists(path)) {
      try {
        Files.createDirectories(path);
      } catch (IOException ex) {
        System.err.println("Failed to create directory: " + path);
        ex.printStackTrace(System.err);
      }
    }
  }

  private static CommandLine getCommandLine(String[] args) throws IOException {
    try {
      Options options = new Options();
      options.addOption(
          Option.builder()
              .longOpt(INPUT_TILE_ARG)
              .hasArg(true)
              .argName("file")
              .desc("Path to the input MVT file")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(INPUT_MBTILES_ARG)
              .hasArg(true)
              .argName("file")
              .desc("Path of the input MBTiles file.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(INPUT_OFFLINEDB_ARG)
              .hasArg(true)
              .argName("file")
              .desc("Path of the input offline database file.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(OUTPUT_DIR_ARG)
              .hasArg(true)
              .argName("dir")
              .desc(
                  "Directory where the output is written, using the input file basename (OPTIONAL).")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(OUTPUT_FILE_ARG)
              .hasArg(true)
              .argName("file")
              .desc(
                  "Filename where the output will be written. Overrides --" + OUTPUT_DIR_ARG + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(EXCLUDE_IDS_OPTION)
              .hasArg(false)
              .desc("Don't include feature IDs.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(FILTER_LAYERS_OPTION)
              .hasArg(true)
              .desc("Filter layers by regex")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(FILTER_LAYERS_INVERT_OPTION)
              .hasArg(false)
              .desc("Invert the result of --" + FILTER_LAYERS_OPTION)
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(INCLUDE_METADATA_OPTION)
              .hasArg(false)
              .desc(
                  "Write tile metadata alongside the output tile (adding '.meta'). "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(ALLOW_COERCE_OPTION)
              .hasArg(false)
              .desc("Allow coercion of property values")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(ALLOW_ELISION_OPTION)
              .hasArg(false)
              .desc("Allow elision of mismatched property types")
              .required(false)
              .get());

      options.addOption(
          Option.builder()
              .longOpt(INCLUDE_TILESET_METADATA_OPTION)
              .hasArg(false)
              .desc(
                  "Write tileset metadata JSON alongside the output tile (adding '.json'). "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(ADVANCED_ENCODING_OPTION)
              .hasArg(false)
              .desc("Enable advanced encodings (FSST & FastPFOR).")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(NO_MORTON_OPTION)
              .hasArg(false)
              .desc("Disable Morton encoding.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(PRE_TESSELLATE_OPTION)
              .hasArg(false)
              .desc("Include tessellation data in converted tiles.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(TESSELLATE_URL_OPTION)
              .hasArg(true)
              .desc("Use a tessellation server (implies --" + PRE_TESSELLATE_OPTION + ").")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(OUTLINE_FEATURE_TABLES_OPTION)
              .hasArgs()
              .desc(
                  "The feature tables for which outlines are included "
                      + "([OPTIONAL], comma-separated, 'ALL' for all, default: none).")
              .valueSeparator(',')
              .argName("tables")
              .required(false)
              .optionalArg(false)
              .get());
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
              .get());
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
              .get());
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
              .get());
      options.addOption(
          Option.builder()
              .longOpt(COMPARE_GEOM_OPTION)
              .hasArg(false)
              .desc(
                  "Assert that geometry in the decoded tile is the same as the input tile. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(COMPARE_PROP_OPTION)
              .hasArg(false)
              .desc(
                  "Assert that properties in the decoded tile is the same as the input tile. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(COMPARE_ALL_OPTION)
              .hasArg(false)
              .desc("Equivalent to --" + COMPARE_GEOM_OPTION + " --" + COMPARE_PROP_OPTION + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(VECTORIZED_OPTION)
              .hasArg(false)
              .desc("Use the vectorized decoding path.")
              .required(false)
              .deprecated()
              .get());
      options.addOption(
          Option.builder()
              .longOpt(DUMP_STREAMS_OPTION)
              .hasArg(false)
              .desc(
                  "Dump the raw contents of the individual streams. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(TIMER_OPTION)
              .hasArg(false)
              .desc("Print the time it takes, in ms, to decode a tile.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(COMPRESS_OPTION)
              .hasArg(true)
              .argName("algorithm")
              .desc(
                  "Compress tile data with one of 'deflate', 'gzip'. "
                      + "Only applies to MBTiles and offline databases.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .option("v")
              .longOpt(VERBOSE_OPTION)
              .hasArg(false)
              .desc("Enable verbose output.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .option("h")
              .longOpt(HELP_OPTION)
              .hasArg(false)
              .desc("Show this output.")
              .required(false)
              .get());

      var cmd = new DefaultParser().parse(options, args);

      var tessellateSource = cmd.getOptionValue(TESSELLATE_URL_OPTION, (String) null);
      if (tessellateSource != null) {
        // throw if it's not a valid URI
        var ignored = new URI(tessellateSource);
      }

      final var filterRegex = cmd.getOptionValue(FILTER_LAYERS_OPTION, (String) null);
      if (filterRegex != null) {
        // throw if it's not a valid regex
        var ignored = Pattern.compile(filterRegex);
      }

      if (cmd.getOptions().length == 0 || cmd.hasOption(HELP_OPTION)) {
        final var autoUsage = true;
        final var header =
            "\nConvert an MVT tile file or MBTiles containing MVT tiles to MLT format.\n\n";
        final var footer = "";
        final var formatter = HelpFormatter.builder().get();
        formatter.printHelp(Encode.class.getName(), header, options, footer, autoUsage);
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
    } catch (IOException | ParseException ex) {
      System.err.println("Failed to parse options: " + ex.getMessage());
    } catch (URISyntaxException e) {
      System.err.println("Invalid tessellation URL: " + e.getMessage());
    }
    return null;
  }

  private static long totalCompressedInput = 0;
  private static long totalCompressedOutput = 0;

  private static final String MetadataMIMEType = "application/vnd.maplibre-vector-tile";
}
