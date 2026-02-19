package org.maplibre.mlt.cli;

import com.onthegomap.planetiler.archive.TileArchiveMetadata;
import com.onthegomap.planetiler.archive.TileArchiveWriter;
import com.onthegomap.planetiler.archive.TileCompression;
import com.onthegomap.planetiler.archive.TileEncodingResult;
import com.onthegomap.planetiler.archive.TileFormat;
import com.onthegomap.planetiler.archive.WriteableTileArchive.TileWriter;
import com.onthegomap.planetiler.geo.TileCoord;
import com.onthegomap.planetiler.pmtiles.Pmtiles.Compression;
import com.onthegomap.planetiler.pmtiles.Pmtiles.TileType;
import com.onthegomap.planetiler.pmtiles.ReadablePmtiles;
import com.onthegomap.planetiler.pmtiles.WriteablePmtiles;
import io.tileverse.rangereader.RangeReader;
import io.tileverse.rangereader.RangeReaderFactory;
import io.tileverse.rangereader.cache.CachingRangeReader;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.net.URI;
import java.nio.file.Path;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Function;
import java.util.function.Supplier;
import org.apache.commons.lang3.mutable.MutableBoolean;
import org.jetbrains.annotations.NotNull;

public class PMTilesHelper extends ConversionHelper {
  /// Encode the MVT tiles in a PMTiles file
  static boolean encodePMTiles(URI inputURI, Path outputPath, EncodeConfig config)
      throws IOException {

    final var readerCacheWeight = 250 * 1024 * 1024;
    try (final var rawReader = RangeReaderFactory.create(inputURI);
        final var cachingReader =
            CachingRangeReader.builder(rawReader).maximumWeight(readerCacheWeight).build()) {

      // Although `RangeReader` is thread-safe, `ReadablePmtiles` is not because
      // it uses a single `Channel` with separate `position` and `read` operations.
      // So use a separate instance for each thread, but share the underlying reader and cache.
      final var threadCount = config.taskRunner().getThreadCount();
      final var readers = new ConcurrentHashMap<Long, Optional<ReadablePmtiles>>(1 + threadCount);
      final Supplier<Optional<ReadablePmtiles>> readerSupplier =
          () ->
              readers.computeIfAbsent(
                  Thread.currentThread().threadId(),
                  ignored -> tryCreateReadablePmtiles(cachingReader));
      final var maybeReader = readerSupplier.get();
      if (maybeReader.isEmpty()) {
        System.err.printf("ERROR: Failed to read PMTiles from '%s'%n", inputURI);
        return false;
      }

      final var reader = maybeReader.get();

      if (config.verboseLevel() > 1) {
        System.err.printf("Opened '%s'%n", inputURI);
      }

      final var header = reader.getHeader();
      if (header.tileType() != TileType.MVT) {
        System.err.printf(
            "ERROR: Input PMTiles tile type is %d, expected %d (MVT)%n",
            header.tileType(), TileType.MVT);
        return false;
      }

      // If a compression type is given (including none) try to use that, otherwise
      // use the source compression type mapped to supported types.
      final TileCompression targetCompressType =
          (config.compressionType() == null)
              ? toTileCompression(header.tileCompression())
              : TileCompression.fromId(config.compressionType());

      final int actualMinZoom = Math.max(header.minZoom(), config.minZoom());
      final int actualMaxZoom = Math.min(header.maxZoom(), config.maxZoom());

      final var oldMetadata = reader.metadata();
      final var newMetadata =
          new TileArchiveMetadata(
              oldMetadata.name(),
              oldMetadata.description(),
              oldMetadata.attribution(),
              oldMetadata.version(),
              oldMetadata.type(),
              TileFormat.MLT,
              oldMetadata.bounds(),
              oldMetadata.center(),
              actualMinZoom,
              actualMaxZoom,
              oldMetadata.json(),
              oldMetadata.others(),
              targetCompressType);

      try (final var fileWriter = WriteablePmtiles.newWriteToFile(outputPath);
          final var tileWriter = fileWriter.newTileWriter()) {
        if (config.verboseLevel() > 1) {
          System.err.printf("Opened '%s'%n", outputPath);
        }

        final var taskRunner = config.taskRunner();
        final var success = new AtomicBoolean(true);
        final var tilesProcessed = new AtomicLong(0);
        final var totalTileCount = new AtomicLong(0);

        final var state =
            new ConversionState(
                config,
                tilesProcessed,
                totalTileCount,
                new AtomicBoolean(false), // directoryComplete
                success);

        final var processTile = getProcessTileFunction(readerSupplier, tileWriter, state);

        taskRunner.run(
            () -> {
              processTiles(
                  taskRunner,
                  processTile,
                  readerSupplier,
                  state,
                  config.minZoom(),
                  config.maxZoom());
            });

        try {
          // Wait for all tasks to finish
          taskRunner.awaitTermination();
        } catch (InterruptedException e) {
          System.err.println("ERROR : interrupted");
          success.set(false);
        }

        if (success.get()) {
          if (config.verboseLevel() > 0) {
            System.err.printf("%nWriting PMTiles...%n");
          }
          fileWriter.finish(newMetadata);

          if (config.verboseLevel() > 1) {
            System.err.println(fileWriter.toString());
          }
          if (config.verboseLevel() > 0) {
            System.err.printf("Done%n");
          }
        }
        return success.get();
      } catch (IOException ex) {
        System.err.println("ERROR: PMTiles conversion failed: " + ex.getMessage());
        logErrorStack(ex, config.verboseLevel());
        return false;
      }
    }
  }

  private static void processTiles(
      @NotNull TaskRunner taskRunner,
      @NotNull final Function<TileCoord, Boolean> processTile,
      @NotNull final Supplier<Optional<ReadablePmtiles>> readerSupplier,
      @NotNull final ConversionState state,
      final int minZoom,
      final int maxZoom) {
    final var maybeReader = readerSupplier.get();
    if (maybeReader.isEmpty()) {
      System.err.println("ERROR: Failed to read PMTiles");
      state.success.set(false);
      return;
    }
    final var reader = maybeReader.get();
    reader.getAllTileCoords().stream()
        .filter(tc -> minZoom <= tc.z() && tc.z() <= maxZoom)
        .forEach(
            tileCoord -> {
              state.totalTileCount.incrementAndGet();
              if (state.encodeConfig().continueOnError() || state.success.get()) {
                try {
                  taskRunner.run(
                      () -> {
                        if (!processTile.apply(tileCoord)) {
                          state.success.set(false);
                        }
                      });
                } catch (RejectedExecutionException ignored) {
                  // This indicates the thread pool has been shut down due to an error
                }
              }
            });
    state.directoryComplete.set(true);
    taskRunner.shutdown();
  }

  /// Produce a callable function for processing a single tile, capturing everything
  /// it needs, suitable for submitting to a thread pool
  private static Function<TileCoord, Boolean> getProcessTileFunction(
      @NotNull final Supplier<Optional<ReadablePmtiles>> readerSupplier,
      @NotNull final TileWriter writer,
      @NotNull final ConversionState state) {
    return (tileCoord) -> {
      final var tileLabel = String.format("%d:%d,%d", tileCoord.z(), tileCoord.x(), tileCoord.y());
      final var tileCount = state.tilesProcessed.incrementAndGet();

      if (state.encodeConfig().verboseLevel() == 0) {
        if (!state.directoryComplete.get()) {
          // Still fetching tile coordinates
          if (tileCount < 2 || (tileCount % 1000 == 0)) {
            System.err.printf("\rProcessing tile %d         \r", tileCount);
          }
        } else {
          final var totalTiles = state.totalTileCount.get();
          final var progress = 100.0 * tileCount / totalTiles;
          final var prevProgress = 100.0 * Math.max(0, tileCount - 1) / totalTiles;
          if ((tileCount % 10000 == 0)
              || (int) Math.round(progress * 10.0) != (int) Math.round(prevProgress * 10.0)) {
            System.err.printf(
                "\rProcessing tiles: %d / %d  (%.1f%%)       \r", tileCount, totalTiles, progress);
          }
        }
      }

      final var maybeReader = readerSupplier.get();
      if (maybeReader.isEmpty()) {
        System.err.println("ERROR: Failed to read PMTiles");
        return false;
      }

      final var reader = maybeReader.get();
      byte[] tileData = reader.getTile(tileCoord);
      if (tileData == null) {
        if (state.encodeConfig().verboseLevel() > 1) {
          System.err.printf("%s :  No tile data present%n", tileLabel);
        }
        return false;
      }

      final var rawSize = tileData.length;
      try {
        tileData = decompress(new ByteArrayInputStream(tileData));
      } catch (IOException ex) {
        System.err.printf("ERROR : Failed to decompress tile %s: %s%n", tileLabel, ex.getMessage());
        logErrorStack(ex, state.encodeConfig().verboseLevel());
        return false;
      }

      if (state.encodeConfig().verboseLevel() > 0) {
        final var extra =
            (rawSize != tileData.length) ? String.format(", %d compressed", rawSize) : "";
        System.err.printf("%s : Loaded (%d bytes%s)%n", tileLabel, tileData.length, extra);
      }

      final MutableBoolean didCompress = null;
      final var mltData =
          Encode.convertTile(
              tileCoord.x(),
              tileCoord.y(),
              tileCoord.z(),
              tileData,
              state.encodeConfig(),
              Optional.empty(), // compress even if it increases the size
              Optional.empty(),
              didCompress);

      if (mltData != null && mltData.length > 0) {
        final var hash = TileArchiveWriter.generateContentHash(mltData);
        synchronized (writer) {
          writer.write(new TileEncodingResult(tileCoord, mltData, OptionalLong.of(hash)));
        }
      } else if (state.encodeConfig().verboseLevel() > 1) {
        System.err.printf("  Converted tile is empty, skipping%n");
      }
      return true;
    };
  }

  private static TileCompression toTileCompression(Compression compression) {
    return switch (compression) {
      case NONE -> TileCompression.NONE;
      case GZIP -> TileCompression.GZIP;
      default -> {
        throw new IllegalArgumentException("Unsupported PMTiles compression type: " + compression);
      }
    };
  }

  private static Optional<ReadablePmtiles> tryCreateReadablePmtiles(RangeReader reader) {
    try {
      return Optional.of(new ReadablePmtiles(reader.asByteChannel()));
    } catch (IOException ignored) {
    }
    return Optional.empty();
  }

  private record ConversionState(
      @NotNull EncodeConfig encodeConfig,
      @NotNull AtomicLong tilesProcessed,
      @NotNull AtomicLong totalTileCount,
      @NotNull AtomicBoolean directoryComplete,
      @NotNull AtomicBoolean success) {}
}
