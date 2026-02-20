package org.maplibre.mlt.cli;

import jakarta.annotation.Nullable;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.IOException;
import java.io.PrintStream;
import java.net.URI;
import java.net.URISyntaxException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.InvalidPathException;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.ParseException;
import org.apache.commons.io.FilenameUtils;
import org.apache.commons.lang3.mutable.MutableBoolean;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.compare.CompareHelper;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.FeatureTableOptimizations;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.converter.MLTStreamObserverDefault;
import org.maplibre.mlt.converter.MLTStreamObserverFile;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.encodings.fsst.FsstEncoder;
import org.maplibre.mlt.converter.encodings.fsst.FsstJni;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.decoder.MltDecoder;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class Encode {

  public static void main(String[] args) {
    if (!run(args)) {
      System.exit(1);
    }
  }

  public static boolean run(String[] args) {
    try {
      final var cmd = EncodeCommandLine.getCommandLine(args);
      if (cmd == null) {
        return false;
      }

      if (cmd.hasOption(EncodeCommandLine.SERVER_ARG)) {
        return new Server()
            .run(Integer.parseInt(cmd.getOptionValue(EncodeCommandLine.SERVER_ARG, "3001")));
      }

      return run(cmd);
    } catch (Exception e) {
      System.err.println("Failed:");
      e.printStackTrace(System.err);
      return false;
    }
  }

  private static boolean run(CommandLine cmd)
      throws URISyntaxException, IOException, ClassNotFoundException, ParseException {
    final var tileFileNames = cmd.getOptionValues(EncodeCommandLine.INPUT_TILE_ARG);
    final var sortFeaturesPattern =
        cmd.hasOption(EncodeCommandLine.SORT_FEATURES_OPTION)
            ? Pattern.compile(cmd.getOptionValue(EncodeCommandLine.SORT_FEATURES_OPTION, ".*"))
            : null;
    final var regenIDsPattern =
        cmd.hasOption(EncodeCommandLine.REGEN_IDS_OPTION)
            ? Pattern.compile(cmd.getOptionValue(EncodeCommandLine.REGEN_IDS_OPTION, ".*"))
            : null;
    final var outlineFeatureTables =
        cmd.getOptionValues(EncodeCommandLine.OUTLINE_FEATURE_TABLES_OPTION);
    final var useFSSTJava = cmd.hasOption(EncodeCommandLine.FSST_ENCODING_OPTION);
    final var useFSSTNative = cmd.hasOption(EncodeCommandLine.FSST_NATIVE_ENCODING_OPTION);
    final var tessellateSource =
        cmd.getOptionValue(EncodeCommandLine.TESSELLATE_URL_OPTION, (String) null);
    final var tessellateURI = (tessellateSource != null) ? new URI(tessellateSource) : null;
    final var tessellatePolygons =
        (tessellateSource != null) || cmd.hasOption(EncodeCommandLine.PRE_TESSELLATE_OPTION);
    final var compressionType =
        cmd.getOptionValue(EncodeCommandLine.COMPRESS_OPTION, (String) null);
    final var enableCoerceOnTypeMismatch = cmd.hasOption(EncodeCommandLine.ALLOW_COERCE_OPTION);
    final var enableElideOnTypeMismatch = cmd.hasOption(EncodeCommandLine.ALLOW_ELISION_OPTION);
    final var filterRegex =
        cmd.getOptionValue(EncodeCommandLine.FILTER_LAYERS_OPTION, (String) null);
    final var filterPattern = (filterRegex != null) ? Pattern.compile(filterRegex) : null;
    final var filterInvert = cmd.hasOption(EncodeCommandLine.FILTER_LAYERS_INVERT_OPTION);
    final var columnMappings = EncodeCommandLine.getColumnMappings(cmd);
    final var minZoom = cmd.getParsedOptionValue(EncodeCommandLine.MIN_ZOOM_OPTION, 0L).intValue();
    final var maxZoom =
        cmd.getParsedOptionValue(EncodeCommandLine.MAX_ZOOM_OPTION, (long) Integer.MAX_VALUE)
            .intValue();

    final var verboseLevel =
        cmd.hasOption(EncodeCommandLine.VERBOSE_OPTION)
            ? cmd.getParsedOptionValue(EncodeCommandLine.VERBOSE_OPTION, 1L).intValue()
            : 0;
    final var continueOnError = cmd.hasOption(EncodeCommandLine.CONTINUE_OPTION);

    final var threadCountOption =
        cmd.hasOption(EncodeCommandLine.PARALLEL_OPTION)
            ? cmd.getParsedOptionValue(EncodeCommandLine.PARALLEL_OPTION, 0L).intValue()
            : 1;
    final var threadCount =
        (threadCountOption > 0) ? threadCountOption : Runtime.getRuntime().availableProcessors();
    final var taskRunner = createTaskRunner(threadCount);
    if (taskRunner != null && verboseLevel > 0) {
      System.err.println("Using " + taskRunner.getThreadCount() + " threads");
    }

    if (verboseLevel > 0 && !columnMappings.isEmpty()) {
      System.err.printf("Using Column Mappings:%n%s", columnMappings);
    }

    final var conversionConfig =
        ConversionConfig.builder()
            .includeIds(!cmd.hasOption(EncodeCommandLine.EXCLUDE_IDS_OPTION))
            .useFastPFOR(cmd.hasOption(EncodeCommandLine.FASTPFOR_ENCODING_OPTION))
            .useFSST(useFSSTJava || useFSSTNative)
            .mismatchPolicy(enableCoerceOnTypeMismatch, enableElideOnTypeMismatch)
            .preTessellatePolygons(tessellatePolygons)
            .useMortonEncoding(!cmd.hasOption(EncodeCommandLine.NO_MORTON_OPTION))
            .outlineFeatureTableNames(
                outlineFeatureTables != null ? List.of(outlineFeatureTables) : List.of())
            .layerFilterPattern(filterPattern)
            .layerFilterInvert(filterInvert)
            .integerEncoding(ConversionConfig.IntegerEncodingOption.AUTO)
            .build();
    if (verboseLevel > 0 && outlineFeatureTables != null && outlineFeatureTables.length > 0) {
      System.err.println(
          "Including outlines for layers: " + String.join(", ", outlineFeatureTables));
    }

    if (useFSSTNative) {
      if (!FsstJni.isLoaded() || !FsstEncoder.useNative(true)) {
        final var err = FsstJni.getLoadError();
        System.err.println(
            "Native FSST could not be loaded: "
                + ((err != null) ? err.getMessage() : "(no error)"));
        ConversionHelper.logErrorStack(err, verboseLevel);
      }
    } else if (useFSSTJava) {
      FsstEncoder.useNative(false);
    }

    final var encodeConfig =
        EncodeConfig.builder()
            .columnMappings(columnMappings)
            .conversionConfig(conversionConfig)
            .tessellateSource(tessellateURI)
            .sortFeaturesPattern(sortFeaturesPattern)
            .regenIDsPattern(regenIDsPattern)
            .compressionType(compressionType)
            .minZoom(minZoom)
            .maxZoom(maxZoom)
            .willOutput(
                cmd.hasOption(EncodeCommandLine.OUTPUT_FILE_ARG)
                    || cmd.hasOption(EncodeCommandLine.OUTPUT_DIR_ARG))
            .willDecode(cmd.hasOption(EncodeCommandLine.DECODE_OPTION))
            .willPrintMLT(cmd.hasOption(EncodeCommandLine.PRINT_MLT_OPTION))
            .willPrintMVT(cmd.hasOption(EncodeCommandLine.PRINT_MVT_OPTION))
            .compareProp(
                cmd.hasOption(EncodeCommandLine.COMPARE_PROP_OPTION)
                    || cmd.hasOption(EncodeCommandLine.COMPARE_ALL_OPTION))
            .compareGeom(
                cmd.hasOption(EncodeCommandLine.COMPARE_GEOM_OPTION)
                    || cmd.hasOption(EncodeCommandLine.COMPARE_ALL_OPTION))
            .willTime(cmd.hasOption(EncodeCommandLine.TIMER_OPTION))
            .dumpStreams(cmd.hasOption(EncodeCommandLine.DUMP_STREAMS_OPTION))
            .taskRunner(taskRunner)
            .continueOnError(continueOnError)
            .verboseLevel(verboseLevel)
            .build();

    if (tileFileNames != null && tileFileNames.length > 0) {
      if (tileFileNames.length > 1 && cmd.hasOption(EncodeCommandLine.OUTPUT_FILE_ARG)) {
        throw new IllegalArgumentException(
            "Multiple input files not allowed with single output file, use --"
                + EncodeCommandLine.OUTPUT_DIR_ARG);
      }
      for (var tileFileName : tileFileNames) {
        final var outputPath = getOutputPath(cmd, tileFileName, "mlt");
        if (outputPath == null) {
          continue;
        }

        Path streamPath = null;
        if (encodeConfig.dumpStreams()) {
          final var fileName = MLTStreamObserverFile.sanitizeFilename(tileFileName);
          streamPath = getOutputPath(cmd, fileName, null, true);
        }

        if (verboseLevel > 0) {
          System.err.println("Converting " + tileFileName + " to " + outputPath);
        }

        encodeTile(tileFileName, outputPath, streamPath, encodeConfig);
      }
    } else if (cmd.hasOption(EncodeCommandLine.INPUT_MBTILES_ARG)) {
      // Converting all the tiles in an MBTiles file
      var inputPath = cmd.getOptionValue(EncodeCommandLine.INPUT_MBTILES_ARG);
      var outputPath = getOutputPath(cmd, inputPath, "mlt.mbtiles");
      if (!MBTilesHelper.encodeMBTiles(inputPath, outputPath, encodeConfig)) {
        return false;
      }
    } else if (cmd.hasOption(EncodeCommandLine.INPUT_OFFLINEDB_ARG)) {
      final var inputPath = cmd.getOptionValue(EncodeCommandLine.INPUT_OFFLINEDB_ARG);
      var ext = FilenameUtils.getExtension(inputPath);
      if (!ext.isEmpty()) {
        ext = "." + ext;
      }
      var outputPath = getOutputPath(cmd, inputPath, "mlt" + ext);
      if (!OfflineDBHelper.encodeOfflineDB(Path.of(inputPath), outputPath, encodeConfig)) {
        return false;
      }
    } else if (cmd.hasOption(EncodeCommandLine.INPUT_PMTILES_ARG)) {
      final var inputPath = cmd.getOptionValue(EncodeCommandLine.INPUT_PMTILES_ARG);
      var ext = FilenameUtils.getExtension(inputPath);
      if (!ext.isEmpty()) {
        ext = "." + ext;
      }
      var outputPath = getOutputPath(cmd, inputPath, "mlt" + ext);
      if (outputPath == null) {
        return false;
      }

      final var inputURI = getInputURI(inputPath);

      outputPath = outputPath.toAbsolutePath();
      if (!PMTilesHelper.encodePMTiles(inputURI, outputPath, encodeConfig)) {
        return false;
      }
    }

    if (verboseLevel > 0 && totalCompressedInput > 0) {
      System.err.printf(
          "Compressed %d bytes to %d bytes (%.1f%%)%n",
          totalCompressedInput,
          totalCompressedOutput,
          100 * (double) totalCompressedOutput / totalCompressedInput);
    }
    return true;
  }

  ///  Convert a single tile from an individual file
  private static void encodeTile(
      String tileFileName, @NotNull Path outputPath, @Nullable Path streamPath, EncodeConfig config)
      throws IOException {
    final var willCompare = config.compareProp() || config.compareGeom();
    final var inputTilePath = Paths.get(tileFileName);
    final var decodedMvTile = MvtUtils.decodeMvt(inputTilePath);

    final var willTime = config.willTime();
    final Timer timer = willTime ? new Timer() : null;

    final var isIdPresent = true;
    final var metadata =
        MltConverter.createTilesetMetadata(
            decodedMvTile, config.conversionConfig(), config.columnMappingConfig(), isIdPresent);

    if (config.verboseLevel() > 2) {
      printColumnMappings(metadata, System.err);
    }

    final var targetConfig = applyColumnMappingsToConversionConfig(config, metadata);

    MLTStreamObserver streamObserver = new MLTStreamObserverDefault();
    if (config.dumpStreams()) {
      if (streamPath != null) {
        streamObserver = new MLTStreamObserverFile(streamPath);
        Files.createDirectories(streamPath);
        if (config.verboseLevel() > 1) {
          System.err.println("Writing raw streams to " + streamPath);
        }
      }
    }
    var mlTile =
        MltConverter.convertMvt(
            decodedMvTile, metadata, targetConfig, config.tessellateSource(), streamObserver);
    if (willTime) {
      timer.stop("encoding");
    }

    if (config.willOutput()) {
      if (outputPath != null) {
        if (config.verboseLevel() > 0) {
          System.err.println("Writing converted tile to " + outputPath);
        }

        try {
          Files.write(outputPath, mlTile);
        } catch (IOException ex) {
          System.err.println("ERROR: Failed to write tile");
          ex.printStackTrace(System.err);
        }
      }
    }
    if (config.willPrintMVT()) {
      System.out.write(JsonHelper.toJson(decodedMvTile).getBytes(StandardCharsets.UTF_8));
    }
    var needsDecoding = config.willDecode() || willCompare || config.willPrintMLT();
    if (needsDecoding) {
      if (config.verboseLevel() > 1) {
        System.err.println("Decoding converted tile...");
      }
      if (willTime) {
        timer.restart();
      }

      var decodedTile = MltDecoder.decodeMlTile(mlTile);
      if (willTime) {
        timer.stop("decoding");
      }
      if (config.willPrintMLT()) {
        System.out.write(JsonHelper.toJson(decodedTile).getBytes(StandardCharsets.UTF_8));
      }
      if (willCompare) {
        final CompareHelper.CompareMode mode =
            (config.compareGeom() && config.compareProp())
                ? CompareHelper.CompareMode.All
                : (config.compareGeom()
                    ? CompareHelper.CompareMode.Geometry
                    : CompareHelper.CompareMode.Properties);

        final var result =
            CompareHelper.compareTiles(
                decodedTile,
                decodedMvTile,
                mode,
                targetConfig.getLayerFilterPattern(),
                targetConfig.getLayerFilterInvert());
        if (result.isPresent()) {
          System.err.println("Tiles do not match: " + result);
        } else if (config.verboseLevel() > 0) {
          System.err.println("Tiles match");
        }
      }
    }
  }

  // In batch conversion, we don't have the column names yet when we set up the config, so
  // we need to apply the mappings to an existing immutable config object using the column
  // names from a decoded tile metadata.
  private static ConversionConfig applyColumnMappingsToConversionConfig(
      EncodeConfig config, MltMetadata.TileSetMetadata metadata) {
    final var conversionConfig = config.conversionConfig();
    // If the config already has optimizations, don't modify it
    if (!conversionConfig.getOptimizations().isEmpty()) {
      return conversionConfig;
    }

    // Warn if both patterns match on any tables
    final var warnTables =
        metadata.featureTables.stream()
            .filter(
                table ->
                    config.sortFeaturesPattern() != null
                        && config.sortFeaturesPattern().matcher(table.name).matches())
            .filter(
                table ->
                    config.regenIDsPattern() != null
                        && config.regenIDsPattern().matcher(table.name).matches())
            .map(table -> table.name)
            .collect(Collectors.joining(","));
    if (!warnTables.isEmpty()) {
      System.err.printf(
          "WARNING: --"
              + EncodeCommandLine.SORT_FEATURES_OPTION
              + " and --"
              + EncodeCommandLine.REGEN_IDS_OPTION
              + " are incompatible: "
              + warnTables
              + "%n");
    }

    // Now that we have the actual column names from the metadata, we can determine which column
    // mappings apply to which tables and create the optimizations for each table accordingly.
    final var optimizationMap =
        metadata.featureTables.stream()
            .collect(
                Collectors.toUnmodifiableMap(
                    table -> table.name,
                    table ->
                        new FeatureTableOptimizations(
                            config.sortFeaturesPattern() != null
                                && config.sortFeaturesPattern().matcher(table.name).matches(),
                            config.regenIDsPattern() != null
                                && config.regenIDsPattern().matcher(table.name).matches(),
                            config.columnMappingConfig().entrySet().stream()
                                .filter(entry -> entry.getKey().matcher(table.name).matches())
                                .flatMap(entry -> entry.getValue().stream())
                                .toList())));

    // re-create the config with the applied column mappings
    return conversionConfig.asBuilder().optimizations(optimizationMap).build();
  }

  /**
   * Converts a tile from MVT to MLT, handling de- and re-compression
   *
   * @param x The x-coordinate of the tile.
   * @param y The y-coordinate of the tile.
   * @param z The zoom level of the tile.
   * @param srcTileData The source tile data as a byte array.
   * @param encodeConfig The configuration for the conversion process
   * @param compressionRatioThreshold An optional threshold for the compression ratio to determine
   *     whether compression is worth the need to decompress it. The compressed version will only be
   *     used if its size is less than the original size multiplied by this ratio. If not present, a
   *     compressed result will be used even if it's larger than the original.
   * @param compressionFixedThreshold An optional fixed byte threshold to determine whether
   *     compression is worth the need to decompress it. The compressed version will only be used if
   *     its size is at least this many bytes smaller than the original size. If not present, a
   *     compressed result will be used even if it's larger than the original.
   * @param didCompress A mutable boolean to indicate whether compression was applied
   * @return The converted tile data as a byte array, or null if the conversion failed.
   */
  @Nullable
  static byte[] convertTile(
      long x,
      long y,
      int z,
      @NotNull byte[] srcTileData,
      @NotNull EncodeConfig config,
      @NotNull Optional<Double> compressionRatioThreshold,
      @NotNull Optional<Long> compressionFixedThreshold,
      @Nullable MutableBoolean didCompress) {
    try {
      // Decode the source tile data into an intermediate representation.
      final var decodedMvTile = MvtUtils.decodeMvt(srcTileData);

      final var isIdPresent = true;
      final var metadata =
          MltConverter.createTilesetMetadata(
              decodedMvTile, config.conversionConfig(), config.columnMappingConfig(), isIdPresent);

      // Print column mappings if verbosity level is high.
      if (config.verboseLevel() > 2) {
        printColumnMappings(metadata, System.err);
      }

      // Apply column mappings and update the conversion configuration.
      final var targetConfig = applyColumnMappingsToConversionConfig(config, metadata);

      // Convert the tile using the updated configuration and tessellation source.
      var tileData =
          MltConverter.convertMvt(decodedMvTile, metadata, targetConfig, config.tessellateSource());

      // Apply compression if specified.
      if (config.compressionType() != null) {
        try (var outputStream = new ByteArrayOutputStream()) {
          try (var compressStream =
              ConversionHelper.createCompressStream(outputStream, config.compressionType())) {
            compressStream.write(tileData);
          }

          // Evaluate whether the compressed version is worth using.
          if ((compressionFixedThreshold.isEmpty()
                  || outputStream.size() < tileData.length - compressionFixedThreshold.get())
              && (compressionRatioThreshold.isEmpty()
                  || outputStream.size() < tileData.length * compressionRatioThreshold.get())) {
            if (config.verboseLevel() > 1) {
              System.err.printf(
                  "  Compressed %d:%d,%d from %d to %d bytes (%.1f%%)%n",
                  z,
                  x,
                  y,
                  tileData.length,
                  outputStream.size(),
                  100 * (double) outputStream.size() / tileData.length);
              totalCompressedInput += tileData.length;
              totalCompressedOutput += outputStream.size();
            }
            if (didCompress != null) {
              didCompress.setTrue();
            }
            tileData = outputStream.toByteArray();
          } else {
            if (config.verboseLevel() > 1) {
              System.err.printf(
                  "  Compression of %d:%d,%d not effective, saving uncompressed (%d vs %d bytes)%n",
                  z, x, y, tileData.length, outputStream.size());
            }
            if (didCompress != null) {
              didCompress.setFalse();
            }
          }
        }
      }
      return tileData;
    } catch (IOException ex) {
      // Log an error message if tile conversion fails.
      System.err.printf("ERROR: Failed to process tile (%d:%d,%d): %s%n", z, x, y, ex.getMessage());
      if (config.verboseLevel() > 1) {
        ex.printStackTrace(System.err);
      }
    }
    return null;
  }

  private static void printColumnMappings(
      MltMetadata.TileSetMetadata metadata,
      @SuppressWarnings("SameParameterValue") PrintStream out) {
    for (var table : metadata.featureTables) {
      for (var column : table.columns) {
        if (column.complexType != null
            && column.complexType.physicalType == MltMetadata.ComplexType.STRUCT) {
          out.format(
              "Found column mapping: %s => %s%n",
              table.name,
              column.complexType.children.stream()
                  .map(f -> column.name + f.name)
                  .collect(Collectors.joining(", ")));
        }
      }
    }
  }

  private static URI getInputURI(String inputArg) {
    final var file = new File(inputArg);
    return file.isFile() ? file.getAbsoluteFile().toURI().normalize() : URI.create(inputArg);
  }

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
    if (cmd.hasOption(EncodeCommandLine.OUTPUT_FILE_ARG)) {
      outputPath = Paths.get(cmd.getOptionValue(EncodeCommandLine.OUTPUT_FILE_ARG));
    } else {
      final var outputDir = cmd.getOptionValue(EncodeCommandLine.OUTPUT_DIR_ARG, "./");

      // Get the file basename without extension.  The input may be a local path or a URI (for
      // pmtiles)
      final var inputURI = getInputURI(inputFileName);
      if (inputURI.getPath() == null) {
        System.err.println("ERROR: Unable to determine input filename for output path");
        return null;
      }

      String baseName;
      try {
        final var inputPath = Paths.get(inputURI.getPath());
        baseName = FilenameUtils.getBaseName(inputPath.getFileName().toString());
      } catch (InvalidPathException ignored) {
        //  Windows can't handle getting the path part of a file URI
        baseName = FilenameUtils.getBaseName(inputFileName);
      }

      outputPath = Paths.get(outputDir, baseName + ext);
    }
    if (outputPath != null) {
      if (forceExt) {
        outputPath = Path.of(FilenameUtils.removeExtension(outputPath.toString()) + ext);
      }

      final var outputDir = outputPath.toAbsolutePath().getParent();
      if (!Files.exists(outputDir)) {
        try {
          Files.createDirectories(outputDir);
        } catch (IOException ex) {
          System.err.printf("Failed to create directory '%s': %s", outputDir, ex.getMessage());
          return null;
        }
      }
    }
    return outputPath;
  }

  private static TaskRunner createTaskRunner(int threadCount) {
    if (threadCount <= 1) {
      return new SerialTaskRunner();
    }
    // Create a thread pool with a bounded task queue that will not reject tasks when it's full.
    // Tasks beyond the limit will run on the calling thread, preventing OOM from too many tasks
    // while allowing for parallelism when the pool is available.
    return new ThreadPoolTaskRunner(
        new ThreadPoolExecutor(
            threadCount,
            threadCount,
            100L,
            TimeUnit.MILLISECONDS,
            new LinkedBlockingQueue<>(4 * threadCount),
            new ThreadPoolExecutor.CallerRunsPolicy()));
  }

  private static long totalCompressedInput = 0;
  private static long totalCompressedOutput = 0;
}
