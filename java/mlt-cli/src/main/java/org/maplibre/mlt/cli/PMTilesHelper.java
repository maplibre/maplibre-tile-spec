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
import com.onthegomap.planetiler.util.CloseableIterator;
import io.tileverse.rangereader.RangeReaderFactory;
import jakarta.annotation.Nullable;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.net.URI;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.OptionalLong;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Function;
import java.util.regex.Pattern;
import org.apache.commons.lang3.mutable.MutableBoolean;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.mvt.ColumnMapping;

public class PMTilesHelper {
  /// Encode the MVT tiles in a PMTiles file
  static boolean encodePMTiles(URI inputURI, Path outputPath, EncodeConfig config)
      throws IOException {

    try (final var rawReader = RangeReaderFactory.create(inputURI);
        final var readChannel = rawReader.asByteChannel();
        final var reader = new ReadablePmtiles(readChannel)) {

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

      ;

      try (final var fileWriter = WriteablePmtiles.newWriteToFile(outputPath);
          final var tileWriter = fileWriter.newTileWriter()) {
        if (config.verboseLevel() > 1) {
          System.err.printf("Opened '%s'%n", outputPath);
        }

        final var threadPool = config.threadPool();
        final var success = new AtomicBoolean(true);
        final var tilesProcessed = new AtomicLong(0);
        final var totalTileCount = new AtomicLong(0);

        final var state =
            new ConversionState(
                config,
                tilesProcessed,
                totalTileCount,
                new AtomicBoolean(false), // directoryComplete
                success,
                config.columnMappings(),
                config.conversionConfig(),
                config.tessellateSource(),
                config.sortFeaturesPattern(),
                config.regenIDsPattern(),
                config.continueOnError(),
                config.verboseLevel());

        final var processTile = getProcessTileFunction(reader, tileWriter, state);

        CliUtil.runTask(
            threadPool,
            () -> {
              processTiles(
                  threadPool, processTile, reader, state, config.minZoom(), config.maxZoom());
            });

        if (threadPool != null) {
          try {
            // Wait for all tasks to finish
            threadPool.awaitTermination(Long.MAX_VALUE, TimeUnit.NANOSECONDS);
          } catch (InterruptedException e) {
            System.err.println("ERROR : interrupted");
            success.set(false);
          }
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
        if (config.verboseLevel() > 1) {
          ex.printStackTrace(System.err);
        }
        return false;
      }
    }
  }

  private static void processTiles(
      @Nullable ExecutorService threadPool,
      @NotNull Function<TileCoord, Boolean> processTile,
      @NotNull ReadablePmtiles reader,
      @NotNull ConversionState state,
      int minZoom,
      int maxZoom) {
    final CloseableIterator<TileCoord> tileCoordIterator;
    synchronized (reader) {
      tileCoordIterator = reader.getAllTileCoords();
    }

    while (true) {
      TileCoord tileCoord;
      synchronized (reader) {
        if (!tileCoordIterator.hasNext()) {
          state.directoryComplete.set(true);
          break;
        }
        tileCoord = tileCoordIterator.next();
      }
      if (tileCoord.z() < minZoom || tileCoord.z() > maxZoom) {
        continue;
      }
      state.totalTileCount.incrementAndGet();
      if (state.continueOnError || state.success.get()) {

        // TODO: Limit the number of queued tasks.  Using LinkedBlockingQueue doesn't work.
        try {
          CliUtil.runTask(
              threadPool,
              () -> {
                if (!processTile.apply(tileCoord)) {
                  state.success.set(false);
                }
              });
        } catch (RejectedExecutionException ignored) {
          // This indicates the thread pool has been shut down due to an error
        }
      }
    }
    if (threadPool != null) {
      threadPool.shutdown();
    }
  }

  /// Produce a callable function for processing a single tile,capturing everything
  /// it needs, suitable for submitting to a thread pool
  private static Function<TileCoord, Boolean> getProcessTileFunction(
      @NotNull ReadablePmtiles reader, @NotNull TileWriter writer, @NotNull ConversionState state) {
    return (tileCoord) -> {
      final var tileLabel = String.format("%d:%d,%d", tileCoord.z(), tileCoord.x(), tileCoord.y());
      final var tileCount = state.tilesProcessed.incrementAndGet();

      if (state.verboseLevel == 0) {
        if (!state.directoryComplete.get()) {
          // Still fetching tile coordinates
          System.err.printf("\rProcessing tile %d         \r", tileCount);
        } else {
          System.err.printf(
              "\rProcessing tiles: %d / %d      \r", tileCount, state.totalTileCount.get());
        }
      }

      var tileData = reader.getTile(tileCoord);
      if (tileData == null) {
        if (state.verboseLevel > 1) {
          System.err.printf("%s :  No tile data present%n", tileLabel);
        }
        return false;
      }

      final var rawSize = tileData.length;
      try {
        tileData = CliUtil.decompress(new ByteArrayInputStream(tileData));
      } catch (IOException ex) {
        System.err.printf("ERROR : Failed to decompress tile %s: %s%n", tileLabel, ex.getMessage());
        if (state.verboseLevel > 1) {
          ex.printStackTrace(System.err);
        }
        return false;
      }

      if (state.verboseLevel > 0) {
        final var extra =
            (rawSize != tileData.length) ? String.format(", %d compressed", rawSize) : "";
        System.err.printf("%s : Loaded (%d bytes%s)%n", tileLabel, tileData.length, extra);
      }

      final MutableBoolean didCompress = null; // force compression
      var mltData =
          Encode.convertTile(
              tileCoord.x(),
              tileCoord.y(),
              tileCoord.z(),
              tileData,
              state.encodeConfig(),
              didCompress);

      if (mltData != null && mltData.length > 0) {
        final var hash = TileArchiveWriter.generateContentHash(tileData);
        synchronized (writer) {
          writer.write(new TileEncodingResult(tileCoord, tileData, OptionalLong.of(hash)));
        }
      } else if (state.verboseLevel > 1) {
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

  private record ConversionState(
      @NotNull EncodeConfig encodeConfig,
      @NotNull AtomicLong tilesProcessed,
      @NotNull AtomicLong totalTileCount,
      @NotNull AtomicBoolean directoryComplete,
      @NotNull AtomicBoolean success,
      @NotNull Map<Pattern, List<ColumnMapping>> columnMappings,
      @NotNull ConversionConfig conversionConfig,
      @Nullable URI tessellateSource,
      @Nullable Pattern sortFeaturesPattern,
      @Nullable Pattern regenIDsPattern,
      boolean continueOnError,
      int verboseLevel) {}
}
