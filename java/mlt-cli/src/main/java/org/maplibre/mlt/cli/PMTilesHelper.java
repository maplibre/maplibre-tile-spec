package org.maplibre.mlt.cli;

import com.google.gson.GsonBuilder;
import com.google.gson.JsonObject;
import com.google.gson.JsonSyntaxException;
import io.tileverse.pmtiles.PMTilesHeader;
import io.tileverse.pmtiles.PMTilesReader;
import io.tileverse.pmtiles.PMTilesWriter;
import io.tileverse.rangereader.RangeReaderFactory;
import io.tileverse.rangereader.cache.CachingRangeReader;
import io.tileverse.tiling.pyramid.TileIndex;
import jakarta.annotation.Nullable;
import java.io.IOException;
import java.net.URI;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Future;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.regex.Pattern;
import java.util.stream.IntStream;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.mvt.ColumnMapping;

public class PMTilesHelper {
  // Not yet available in `io.tileverse.pmtiles.PMTilesHeader`
  private static final byte PMT_MLT_TILE_TYPE = 6;

  /// Encode the MVT tiles in a PMTiles file
  static boolean encodePMTiles(URI inputPath, Path outputPath, EncodeConfig config)
      throws IOException {

    try (var rawReader = RangeReaderFactory.create(inputPath);
        var cachingReader =
            CachingRangeReader.builder(rawReader)
                .maximumWeight(50 * 1024 * 1024) // 50MB cache for headers/directories
                .build()) {
      var reader = new PMTilesReader(cachingReader::asByteChannel);
      if (config.verboseLevel() > 1) {
        System.err.printf("Opened '%s'%n", inputPath);
      }

      final var header = reader.getHeader();

      if (header.tileType() != PMTilesHeader.TILETYPE_MVT) {
        System.err.printf(
            "ERROR: Input PMTiles tile type is %d, expected %d (MVT)%n",
            header.tileType(), PMTilesHeader.TILETYPE_MVT);
        return false;
      }

      // If a compression type is given (including none) try to use that, otherwise
      // use the source compression type mapped to supported types.
      final var targetCompressType =
          (config.compressionType() == null)
              ? header.tileCompression()
              : getCompressionType(config.compressionType());

      final int actualMinZoom = Math.max(header.minZoom(), config.minZoom());
      final int actualMaxZoom = Math.min(header.maxZoom(), config.maxZoom());

      // `getMetadata` drops values it doesn't understand, like "format".
      // `PMTilesWriter` doesn't accept a `PMTilesMetadata` anyway.
      var metadataStr = reader.getMetadataAsString();
      try {
        final var gson = new GsonBuilder().create();
        final var metadata = gson.fromJson(metadataStr, JsonObject.class);
        metadata.addProperty("format", "mlt");
        if (config.minZoom() > 0) {
          metadata.addProperty("minzoom", actualMinZoom);
        }
        if (config.maxZoom() < Integer.MAX_VALUE) {
          metadata.addProperty("maxzoom", actualMaxZoom);
        }
        metadataStr = gson.toJson(metadata);
      } catch (JsonSyntaxException ex) {
        // Write the original metadata, unchanged, if we can't parse it
        System.err.printf("WARNING: Failed to parse metadata%n");
      }

      try (var writer =
          PMTilesWriter.builder()
              .outputPath(outputPath)
              .center(header.centerLon(), header.centerLat(), header.centerZoom())
              .minZoom(actualMinZoom)
              .maxZoom(actualMaxZoom)
              .tileCompression(targetCompressType)
              .tileType(PMT_MLT_TILE_TYPE)
              .bounds(header.minLon(), header.minLat(), header.maxLon(), header.maxLat())
              .internalCompression(reader.getHeader().internalCompression())
              .build()) {

        if (config.verboseLevel() > 1) {
          System.err.printf("Opened '%s'%n", outputPath);
        }

        writer.setMetadata(metadataStr);

        final var success = new AtomicBoolean(true);
        final var totalZooms = actualMaxZoom - actualMinZoom + 1;
        final var zoomsToFetch = new AtomicInteger(totalZooms);
        final var tilesProcessed = new AtomicLong(0);
        final var totalTileCount = new AtomicLong(0);

        final Function<Integer, List<TileIndex>> getIndexes =
            (zoom) -> {
              final var indices = reader.getTileIndicesByZoomLevel(zoom).toList();
              if (config.verboseLevel() > 0) {
                final var totalTiles = 1L << (zoom * 2);
                System.err.printf(
                    "Zoom %d : %d / %d tiles (%.1f%%)%n",
                    zoom, indices.size(), totalTiles, 100.0 * indices.size() / totalTiles);
              }
              return indices;
            };

        final var state =
            new ConversionState(
                config,
                totalZooms,
                tilesProcessed,
                totalTileCount,
                zoomsToFetch,
                success,
                config.columnMappings(),
                config.conversionConfig(),
                config.tessellateSource(),
                config.sortFeaturesPattern(),
                config.regenIDsPattern(),
                config.continueOnError(),
                config.verboseLevel());
        final var processTile = getProcessTileFunction(reader, writer, state);
        final var processZoom = getProcessZoomFunction(config.threadPool(), processTile, state);

        // For each zoom level, get the list of tile indices, then convert them in parallel.
        IntStream.rangeClosed(actualMinZoom, actualMaxZoom)
            .mapToObj(Integer::valueOf)
            .map(zoom -> CliUtil.runTask(config.threadPool(), () -> getIndexes.apply(zoom)))
            .forEach(processZoom);

        if (config.threadPool() != null) {
          try {
            // The pool needs to be closed before awaiting termination but, since tasks add
            // other tasks, we can't close it immediately.
            // On success, it will already have been closed after processing the last zoom level.
            CliUtil.joinThreadPool(config.threadPool(), !success.get());
          } catch (InterruptedException e) {
            System.err.println("ERROR : interrupted");
            success.set(false);
          }
        }

        if (success.get()) {
          if (config.verboseLevel() > 0) {
            final var writerListener = new WriterProgressListener();
            writer.setProgressListener(writerListener);
          } else {
            System.err.printf("%nProcessing tiles: 100%%  %nWriting PMTiles...%n");
          }

          // actually write the file
          writer.complete();

          if (config.verboseLevel() > 0) {
            System.err.println("");
          }
        }
        return success.get();
      } catch (IOException ex) {
        System.err.println("ERROR: PMTiles conversion failed: " + ex.getMessage());
        if (config.verboseLevel() > 1) {
          ex.printStackTrace(System.err);
        }
        return false;
      }
    }
  }

  ///  Produce a function for processing a single zoom level, with everything else it needs
  // captured, suitable for submitting to a thread pool
  private static Consumer<Future<List<TileIndex>>> getProcessZoomFunction(
      @Nullable ExecutorService threadPool,
      @NotNull Function<TileIndex, Boolean> processTile,
      @NotNull ConversionState state) {
    return (future) -> {
      if (!state.continueOnError && !state.success.get()) {
        return;
      }
      try {
        final var indices = future.get();
        state.totalTileCount.addAndGet(indices.size());
        final var zoomsRemaining = state.zoomsToFetch.decrementAndGet();

        // Convert tile in the zoom level
        for (var tileIndex : indices) {
          if (!state.continueOnError && !state.success.get()) {
            break;
          }
          if (threadPool != null) {
            threadPool.submit(
                () -> {
                  if (!processTile.apply(tileIndex)) {
                    state.success.set(false);
                  }
                });
          } else {
            if (!processTile.apply(tileIndex)) {
              state.success.set(false);
            }
          }
        }
        if (threadPool != null && zoomsRemaining == 0) {
          // This is the last task that can add more tasks
          threadPool.close();
        }
      } catch (InterruptedException | ExecutionException e) {
        System.err.printf("ERROR : Failed to get tile indexes: %s%n", e.getMessage());
        state.success.set(false);
      }
    };
  }

  ///  Produce a callable function for processing a single tile, with everything
  /// else it needs captured, suitable for submitting to a thread pool
  private static Function<TileIndex, Boolean> getProcessTileFunction(
      @NotNull PMTilesReader reader,
      @NotNull PMTilesWriter writer,
      @NotNull ConversionState state) {
    // PMTilesWriter will handle compression, so `encodeTile` must not compress the tile data
    // itself.
    final var encodeConfig = state.encodeConfig().asBuilder().compressionType(null).build();
    return (tileIndex) -> {
      final var tileLabel = String.format("%d:%d,%d", tileIndex.z(), tileIndex.x(), tileIndex.y());
      final var tileCount = state.tilesProcessed.incrementAndGet();

      try {
        if (state.verboseLevel == 0) {
          if (state.zoomsToFetch.get() == 0) {
            final var progress = 100.0 * tileCount / state.totalTileCount.get();
            if ((int) (progress * 1000.0) != (int) ((progress - 1) * 1000.0)) {
              if (tileCount == 1) {
                System.err.println();
              }
              System.err.printf(
                  "\rProcessing tile %d / %d (%.1f%%)       \r",
                  tileCount, state.totalTileCount.get(), progress);
            }
          } else {
            System.err.printf(
                "\rProcessing zooms %d / %d, tiles: %d / %d    \r",
                state.totalZooms - state.zoomsToFetch.get(),
                state.totalZooms,
                tileCount,
                state.totalTileCount.get());
          }
        }

        final var maybeTile = reader.getTile(tileIndex);
        if (!maybeTile.isPresent()) {
          if (state.verboseLevel > 1) {
            System.err.printf("%s :  No tile data present%n", tileLabel);
          }
          return false;
        }
        var tileBuffer = maybeTile.get();
        if (state.verboseLevel > 1) {
          final var info =
              tileBuffer.hasRemaining()
                  ? String.format("%d bytes", tileBuffer.remaining())
                  : "unknown size";
          System.err.printf("%s : Loaded (%s)%n", tileLabel, info);
        } else if (state.verboseLevel > 0) {
          System.err.printf("%s%n", tileLabel);
        }

        final byte[] mvtData;
        if (tileBuffer.hasRemaining()) {
          mvtData = new byte[tileBuffer.remaining()];
          tileBuffer.get(mvtData);
        } else {
          // Entire buffer capacity, with extra trailing bytes.
          // PBF decoder is fine with that.
          mvtData = tileBuffer.array();
        }

        var mltData =
            Encode.convertTile(
                tileIndex.x(), tileIndex.y(), tileIndex.z(), mvtData, encodeConfig, null);

        if (mltData != null && mltData.length > 0) {
          synchronized (writer) {
            writer.addTile(tileIndex, mltData);
          }
        } else if (state.verboseLevel > 1) {
          System.err.printf("  Converted tile is empty, skipping%n");
        }
        return true;
      } catch (IOException ex) {
        // Exceptions within a `forEach` don't propagate, log it and continue.
        System.err.printf("Failed to get tile '%s': %s%n", tileLabel, ex.getMessage());
        if (state.verboseLevel > 1) {
          ex.printStackTrace(System.err);
        }
        return false;
      }
    };
  }

  /// Print out progress of the PMTilesWriter as it's writing
  private static class WriterProgressListener implements PMTilesWriter.ProgressListener {
    private String lastProgress = "";

    @Override
    public void onProgress(double progress) {
      final var progressStr = String.format("%.1f", 100 * progress);
      if (!progressStr.equals(lastProgress)) {
        lastProgress = progressStr;
        System.err.printf("\rWriting PMTiles : %s%%    ", progressStr);
      }
    }

    @Override
    public boolean isCancelled() {
      return false;
    }
  }

  private static byte getCompressionType(String name) {
    return switch (name) {
      case "none" -> PMTilesHeader.COMPRESSION_NONE;
      case "gzip" -> PMTilesHeader.COMPRESSION_GZIP;
      case "brotli" -> PMTilesHeader.COMPRESSION_BROTLI;
      case "zstd" -> PMTilesHeader.COMPRESSION_ZSTD;
      default -> throw new RuntimeException("Compression type not supported: " + name);
    };
  }

  private record ConversionState(
      @NotNull EncodeConfig encodeConfig,
      int totalZooms,
      @NotNull AtomicLong tilesProcessed,
      @NotNull AtomicLong totalTileCount,
      @NotNull AtomicInteger zoomsToFetch,
      @NotNull AtomicBoolean success,
      @NotNull Map<Pattern, List<ColumnMapping>> columnMappings,
      @NotNull ConversionConfig conversionConfig,
      @Nullable URI tessellateSource,
      @Nullable Pattern sortFeaturesPattern,
      @Nullable Pattern regenIDsPattern,
      boolean continueOnError,
      int verboseLevel) {}
}
