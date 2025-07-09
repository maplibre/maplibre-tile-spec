package com.mlt.cli;

import com.mlt.converter.ConversionConfig;
import com.mlt.converter.FeatureTableOptimizations;
import com.mlt.converter.MltConverter;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.data.MapLibreTile;
import com.mlt.decoder.MltDecoder;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.HashMap;
import java.util.List;
import java.util.Optional;
import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.CommandLineParser;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Option;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;
import org.apache.commons.lang3.NotImplementedException;

public class Encode {

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

  private static final String FILE_NAME_ARG = "mvt";
  private static final String OUTPUT_DIR_ARG = "dir";
  private static final String OUTPUT_FILE_ARG = "mlt";
  private static final String EXCLUDE_IDS_OPTION = "noids";
  private static final String INCLUDE_TILESET_METADATA_OPTION = "metadata";
  private static final String ADVANCED_ENCODING_OPTION = "advanced";
  private static final String NO_MORTON_OPTION = "nomorton";
  private static final String PRE_TESSELLATE_OPTION = "tessellate";
  private static final String OUTLINE_FEATURE_TABLES_OPTION = "outlines";
  private static final String DECODE_OPTION = "decode";
  private static final String PRINT_MLT_OPTION = "printmlt";
  private static final String PRINT_MVT_OPTION = "printmvt";
  private static final String COMPARE_OPTION = "compare";
  private static final String VECTORIZED_OPTION = "vectorized";
  private static final String TIMER_OPTION = "timer";

  public static void main(String[] args) {
    Options options = new Options();
    options.addOption(
        Option.builder(FILE_NAME_ARG)
            .hasArg(true)
            .desc("Path to the input MVT file to read ([REQUIRED])")
            .required(true)
            .build());
    options.addOption(
        Option.builder(OUTPUT_DIR_ARG)
            .hasArg(true)
            .desc(
                "Output directory to write an MLT file to ([OPTIONAL], default: no file is written)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(OUTPUT_FILE_ARG)
            .hasArg(true)
            .desc("Output file to write an MLT file to ([OPTIONAL], default: no file is written)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(EXCLUDE_IDS_OPTION)
            .hasArg(false)
            .desc("Don't include feature IDs ([OPTIONAL, default: false])")
            .required(false)
            .build());
    options.addOption(
        Option.builder(INCLUDE_TILESET_METADATA_OPTION)
            .hasArg(false)
            .desc("Include tileset metadata in the output ([OPTIONAL], default: false)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(ADVANCED_ENCODING_OPTION)
            .hasArg(false)
            .desc("Enable advanced encodings (fsst & fastpfor) ([OPTIONAL], default: false)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(NO_MORTON_OPTION)
            .hasArg(false)
            .desc("Disable Morton encoding ([OPTIONAL], default: false)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(PRE_TESSELLATE_OPTION)
            .hasArg(false)
            .desc("Include tessellation in the tile data ([OPTIONAL], default: false)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(OUTLINE_FEATURE_TABLES_OPTION)
            .hasArgs()
            .desc(
                "The feature tables for which outlines are included ([OPTIONAL], comma-separated, * for all, default: none)")
            .valueSeparator(',')
            .required(false)
            .build());
    options.addOption(
        Option.builder(DECODE_OPTION)
            .hasArg(false)
            .desc("Test decoding the tile after encoding it ([OPTIONAL], default: false)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(PRINT_MLT_OPTION)
            .hasArg(false)
            .desc("Print the MLT tile after encoding it ([OPTIONAL], default: false)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(PRINT_MVT_OPTION)
            .hasArg(false)
            .desc("Print the round tripped MVT tile ([OPTIONAL], default: false)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(COMPARE_OPTION)
            .hasArg(false)
            .desc(
                "Assert that data in the the decoded tile is the same as the data in the input tile ([OPTIONAL], default: false)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(VECTORIZED_OPTION)
            .hasArg(false)
            .desc(
                "Use the vectorized decoding path ([OPTIONAL], default: will use non-vectorized path)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(TIMER_OPTION)
            .hasArg(false)
            .desc("Print the time it takes, in ms, to decode a tile ([OPTIONAL])")
            .required(false)
            .build());
    CommandLineParser parser = new DefaultParser();
    try {
      CommandLine cmd = parser.parse(options, args);
      var fileName = cmd.getOptionValue(FILE_NAME_ARG);
      if (cmd.hasOption(OUTPUT_FILE_ARG) && cmd.hasOption(OUTPUT_DIR_ARG)) {
        throw new ParseException(
            "Cannot specify both '-" + OUTPUT_FILE_ARG + "' and '-" + OUTPUT_DIR_ARG + "' options");
      }
      var willOutput = cmd.hasOption(OUTPUT_FILE_ARG) || cmd.hasOption(OUTPUT_DIR_ARG);
      var includeIds = !cmd.hasOption(EXCLUDE_IDS_OPTION);
      var willIncludeTilesetMetadata = cmd.hasOption(INCLUDE_TILESET_METADATA_OPTION);
      var useAdvancedEncodingSchemes = cmd.hasOption(ADVANCED_ENCODING_OPTION);
      var useMortonEncoding = !cmd.hasOption(NO_MORTON_OPTION);
      var preTessellatePolygons = cmd.hasOption(PRE_TESSELLATE_OPTION);
      var outlineFeatureTables = cmd.getOptionValues(OUTLINE_FEATURE_TABLES_OPTION);
      var willDecode = cmd.hasOption(DECODE_OPTION);
      var willPrintMLT = cmd.hasOption(PRINT_MLT_OPTION);
      var willPrintMVT = cmd.hasOption(PRINT_MVT_OPTION);
      var willCompare = cmd.hasOption(COMPARE_OPTION);
      var willUseVectorized = cmd.hasOption(VECTORIZED_OPTION);
      var willTime = cmd.hasOption(TIMER_OPTION);
      var inputTilePath = Paths.get(fileName);
      var inputTileName = inputTilePath.getFileName().toString();
      var decodedMvTile = MvtUtils.decodeMvt(inputTilePath);

      // No ColumnMapping as support is still buggy:
      // https://github.com/maplibre/maplibre-tile-spec/issues/59
      var columnMappings = Optional.<List<ColumnMapping>>empty();
      var isIdPresent = true;
      var tileMetadata =
          MltConverter.createTilesetMetadata(List.of(decodedMvTile), columnMappings, isIdPresent);

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

      Timer timer = new Timer();
      var mlTile = MltConverter.convertMvt(decodedMvTile, conversionConfig, tileMetadata);
      if (willTime) timer.stop("encoding");
      if (willOutput) {
        Path outputPath = null;
        if (cmd.hasOption(OUTPUT_DIR_ARG)) {
          var outputDir = cmd.getOptionValue(OUTPUT_DIR_ARG);
          var outputTileName = String.format("%s.mlt", inputTileName.split("\\.")[0]);
          outputPath = Paths.get(outputDir, outputTileName);
        } else if (cmd.hasOption(OUTPUT_FILE_ARG)) {
          outputPath = Paths.get(cmd.getOptionValue(OUTPUT_FILE_ARG));
        }

        if (outputPath != null) {
          var outputDirPath = outputPath.toAbsolutePath().getParent();
          if (!Files.exists(outputDirPath)) {
            System.out.println("Creating directory: " + outputDirPath);
            Files.createDirectories(outputDirPath);
          }
          System.out.println("Writing converted tile to " + outputPath);
          Files.write(outputPath, mlTile);

          if (willIncludeTilesetMetadata) {
            Path outputMetadataPath = Paths.get(outputPath.toString() + ".meta.pbf");
            System.out.println("Writing metadata to " + outputMetadataPath);
            tileMetadata.writeTo(Files.newOutputStream(outputMetadataPath));
          }
        }
      }
      if (willPrintMVT) {
        printMVT(decodedMvTile);
      }
      var needsDecoding = willDecode || willCompare || willPrintMLT;
      if (needsDecoding) {
        timer.restart();
        // convert MLT wire format to an MapLibreTile object
        if (willUseVectorized) {
          throw new NotImplementedException("Vectorized encoding is not available");
        } else {
          var decodedTile = MltDecoder.decodeMlTile(mlTile, tileMetadata);
          if (willTime) timer.stop("decoding");
          if (willPrintMLT) {
            CliUtil.printMLT(decodedTile);
          }
          if (willCompare) {
            compare(decodedTile, decodedMvTile);
          }
        }
      }
    } catch (Exception e) {
      System.err.println("Failed:");
      e.printStackTrace(System.err);
      System.exit(1);
    }
  }
}
