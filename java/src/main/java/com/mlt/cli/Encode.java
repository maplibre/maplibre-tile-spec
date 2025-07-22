package com.mlt.cli;

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
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.zip.Inflater;
import java.util.zip.InflaterInputStream;
import javax.annotation.Nullable;
import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.HelpFormatter;
import org.apache.commons.cli.Option;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;
import org.apache.commons.compress.compressors.deflate.DeflateCompressorOutputStream;
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream;
import org.apache.commons.compress.compressors.gzip.GzipCompressorOutputStream;
import org.apache.commons.compress.compressors.gzip.GzipParameters;
import org.apache.commons.io.EndianUtils;
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
      var compress = cmd.getOptionValue(COMPRESS_OPTION, (String) null);
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
      } else {
        // Converting all the tiles in an MBTiles file
        var inputMBTilesPath = cmd.getOptionValue(INPUT_MBTILES_ARG);
        var outputPath = getOutputPath(cmd, inputMBTilesPath);
        encodeMBTiles(
            inputMBTilesPath,
            outputPath,
            columnMappings,
            conversionConfig,
            tessellateURI,
            compress,
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
    var outputPath = getOutputPath(cmd, inputTileName);
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
      String compress,
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
                compress,
                verbose);
          }

          // mbtiles4j doesn't support types other than png and jpg,
          // so we have to set the format metadata the hard way.
          var dbFile = mbTilesWriter.close();
          try (var connection =
              DriverManager.getConnection("jdbc:sqlite:" + dbFile.getAbsolutePath())) {
            // The sqlite3 JDBC driver only supports batch execution without prepared statements?!
            try (var statement = connection.createStatement()) {
              statement.addBatch(
                  "UPDATE metadata SET value = 'application/vnd.maplibre-tile-pbf' WHERE name = 'format'");
              statement.addBatch("VACUUM");
              statement.executeBatch();
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
      System.err.println("ERROR: MBTiles conversion failed");
      ex.printStackTrace(System.err);
    } finally {
      if (mbTilesReader != null) {
        mbTilesReader.close();
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
      String compress,
      boolean verbose)
      throws IOException, MBTilesWriteException {
    var x = tile.getColumn();
    var y = tile.getRow();
    var z = tile.getZoom();

    var srcTileData = getTileData(tile);
    var decodedMvTile = MvtUtils.decodeMvt(srcTileData);

    var isIdPresent = true;
    var tileMetadata =
        MltConverter.createTilesetMetadata(List.of(decodedMvTile), columnMappings, isIdPresent);

    var mlTile =
        MltConverter.convertMvt(decodedMvTile, tileMetadata, conversionConfig, tessellateSource);

    byte[] tileData = null;
    try (var outputStream = new ByteArrayOutputStream()) {
      try (var compressStream = compressStream(outputStream, compress)) {
        byte[] binaryMetadata;
        try (var metadataStream = new ByteArrayOutputStream()) {
          tileMetadata.writeTo(metadataStream);
          metadataStream.close();
          binaryMetadata = metadataStream.toByteArray();
        }
        if (binaryMetadata.length >= 1 << 16) {
          throw new NumberFormatException("Invalid metadata size");
        }

        // Can't use auto close, DataOutputStream closes the target stream
        var binStream = new DataOutputStream(compressStream);
        try {
          binStream.writeShort(EndianUtils.swapShort((short) binaryMetadata.length));
          binStream.flush();

          compressStream.write(binaryMetadata);
          compressStream.write(mlTile);
        } finally {
          binStream.close();
        }
      }
      outputStream.close();
      tileData = outputStream.toByteArray();
    } catch (IOException ex) {
      System.err.printf(
          "ERROR: Failed to write tile with embedded PBF metadata (%d:%d,%d): %s\n",
          z, x, y, ex.getMessage());
    }

    if (tileData != null) {
      mbTilesWriter.addTile(tileData, z, x, y);
    }

    if (verbose) {
      System.err.printf("Added %d:%d,%d%n", z, x, y);
    }
  }

  private static OutputStream compressStream(OutputStream src, @NotNull String compress) {
    if (Objects.equals(compress, "gzip")) {
      try {
        var parameters = new GzipParameters();
        parameters.setCompressionLevel(9);
        return new GzipCompressorOutputStream(src, parameters);
      } catch (IOException ignore) {
      }
    }
    if (Objects.equals(compress, "deflate")) {
      return new DeflateCompressorOutputStream(src);
    }
    return src;
  }

  private static byte[] getTileData(Tile tile) throws IOException {
    var srcTileStream = tile.getData();

    // If the tile data is compressed, decompress it
    try {
      InputStream decompressInputStream = null;
      if (srcTileStream.available() > 1) {
        byte byte0 = (byte) srcTileStream.read();
        if (byte0 == 0x78) {
          // raw deflate
          decompressInputStream =
              new InflaterInputStream(srcTileStream, new Inflater(/* nowrap= */ true));
        } else if (byte0 == 0x1f && srcTileStream.available() > 1) {
          byte byte1 = (byte) srcTileStream.read();
          srcTileStream.reset();
          if (byte1 == (byte) 0x8b) {
            // TODO: why doesn't InflaterInputStream / GZIPInputStream work here?
            // decompressInputStream = new GZIPInputStream(srcTileStream);
            decompressInputStream = new GzipCompressorInputStream(srcTileStream);
          }
        } else {
          srcTileStream.reset();
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
      System.err.printf("Failed to decompress tile: %s", ex.getMessage());
    }

    srcTileStream.reset();
    return srcTileStream.readAllBytes();
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

  private static Path getOutputPath(CommandLine cmd, String inputFileName) {
    Path outputPath = null;
    if (cmd.hasOption(OUTPUT_DIR_ARG)) {
      var outputDir = cmd.getOptionValue(OUTPUT_DIR_ARG);
      var ext = cmd.hasOption(INPUT_MBTILES_ARG) ? ".mlt.mbtiles" : ".mlt";
      outputPath = Paths.get(outputDir, inputFileName.split("\\.")[0] + ext);
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
              .desc("Path to the input MBTiles file")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(OUTPUT_DIR_ARG)
              .hasArg(true)
              .argName("dir")
              .desc(
                  "Output directory to write an MLT file to "
                      + "([OPTIONAL], default: no file is written)")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(OUTPUT_FILE_ARG)
              .hasArg(true)
              .argName("file")
              .desc(
                  "Output file to write an MLT tile or MBTiles to"
                      + "([OPTIONAL], default: no file is written)")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(EXCLUDE_IDS_OPTION)
              .hasArg(false)
              .desc("Don't include feature IDs")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(INCLUDE_TILESET_METADATA_OPTION)
              .hasArg(false)
              .desc("Write tileset metadata (PBF) alongside the output (adding '.meta.pbf')")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(INCLUDE_EMBEDDED_METADATA)
              .hasArg(true)
              .argName("file")
              .desc("Write output with embedded metadata ([OPTIONAL])")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(INCLUDE_EMBEDDED_PBF_METADATA)
              .hasArg(true)
              .argName("file")
              .desc("Write output with embedded PBF metadata ([OPTIONAL])")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(ADVANCED_ENCODING_OPTION)
              .hasArg(false)
              .desc("Enable advanced encodings (FSST & FastPFOR)")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(NO_MORTON_OPTION)
              .hasArg(false)
              .desc("Disable Morton encoding")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(PRE_TESSELLATE_OPTION)
              .hasArg(false)
              .desc("Include tessellation data in the tile")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(TESSELLATE_URL_OPTION)
              .hasArg(true)
              .desc("Use a tessellation server (implies --" + PRE_TESSELLATE_OPTION + ")")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(OUTLINE_FEATURE_TABLES_OPTION)
              .hasArgs()
              .desc(
                  "The feature tables for which outlines are included "
                      + "([OPTIONAL], comma-separated, * for all, default: none)")
              .valueSeparator(',')
              .argName("tables")
              .required(false)
              .optionalArg(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(DECODE_OPTION)
              .hasArg(false)
              .desc("Test decoding the tile after encoding it")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(PRINT_MLT_OPTION)
              .hasArg(false)
              .desc("Print the MLT tile after encoding it")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(PRINT_MVT_OPTION)
              .hasArg(false)
              .desc("Print the round-tripped MVT tile")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(COMPARE_OPTION)
              .hasArg(false)
              .desc("Assert that data in the the decoded tile is the same as the input tile")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(VECTORIZED_OPTION)
              .hasArg(false)
              .desc("Use the vectorized decoding path")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(TIMER_OPTION)
              .hasArg(false)
              .desc("Print the time it takes, in ms, to decode a tile")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .longOpt(COMPRESS_OPTION)
              .hasArg(true)
              .argName("algorithm")
              .desc("Compress tile data with one of 'deflate', 'gzip'")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .option("v")
              .longOpt(VERBOSE_OPTION)
              .hasArg(false)
              .desc("Enable verbose output")
              .required(false)
              .build());
      options.addOption(
          Option.builder()
              .option("h")
              .longOpt(HELP_OPTION)
              .hasArg(false)
              .desc("Show this output")
              .required(false)
              .build());

      var cmd = new DefaultParser().parse(options, args);

      var tessellateSource = cmd.getOptionValue(TESSELLATE_URL_OPTION, (String) null);
      if (tessellateSource != null) {
        new URI(tessellateSource);
      }

      if (cmd.getOptions().length == 0 || cmd.hasOption(HELP_OPTION)) {
        new HelpFormatter().printHelp(Encode.class.getName(), options);
      } else if (cmd.hasOption(INPUT_TILE_ARG) == cmd.hasOption(INPUT_MBTILES_ARG)) {
        System.err.println(
            "Specify one of '--" + INPUT_TILE_ARG + "' and '--" + INPUT_MBTILES_ARG + "'");
      } else if (cmd.hasOption(OUTPUT_FILE_ARG) && cmd.hasOption(OUTPUT_DIR_ARG)) {
        System.err.println(
            "Cannot specify both '-" + OUTPUT_FILE_ARG + "' and '-" + OUTPUT_DIR_ARG + "' options");
      } else if (!new HashSet<>(
              Arrays.asList(COMPRESS_OPTION_GZIP, COMPRESS_OPTION_DEFLATE, COMPRESS_OPTION_NONE))
          .contains(cmd.getOptionValue(COMPRESS_OPTION, COMPRESS_OPTION_NONE))) {
        System.err.println("Invalid compression type");
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
}
