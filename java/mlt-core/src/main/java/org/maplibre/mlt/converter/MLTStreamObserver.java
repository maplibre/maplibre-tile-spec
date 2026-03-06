package org.maplibre.mlt.converter;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.Arrays;
import java.util.Collection;
import java.util.stream.IntStream;
import java.util.stream.LongStream;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.util.ByteBufferUtil;

/** Interface for observing streams of encoded data in the MapLibre MLT conversion process. */
public interface MLTStreamObserver {

  /**
   * Checks if the recorder is currently active.
   *
   * @return true if the recorder is active, false otherwise.
   */
  boolean isActive();

  /**
   * Sets the name of the layer associated with the stream recorder.
   *
   * @param layerName the name of the layer to set.
   */
  void setLayerName(String layerName);

  /**
   * Observe a stream of encoded data
   *
   * @param <T> The type of the values in the stream.
   * @param streamName The name of the stream
   * @param values The source values represented by the stream
   * @param rawMetaData The encoded metadata associated with the stream
   * @param rawData The encoded data of the stream
   */
  <T> void observeStream(
      String streamName, Collection<T> values, ByteBuffer rawMetaData, ByteBuffer rawData)
      throws IOException;

  default <T> void observeStream(@NotNull String streamName, @NotNull T[] values)
      throws IOException {
    if (isActive()) {
      observeStream(streamName, Arrays.asList(values), emptyBuffer, emptyBuffer);
    }
  }

  default <T> void observeStream(
      @NotNull String streamName,
      @NotNull T[] values,
      @Nullable byte[] rawMetaData,
      @Nullable byte[] rawData)
      throws IOException {
    if (isActive()) {
      observeStream(streamName, Arrays.asList(values), maybeWrap(rawMetaData), maybeWrap(rawData));
    }
  }

  default <T> void observeStream(
      @NotNull String streamName,
      @NotNull T[] values,
      @NotNull ByteBuffer rawMetaData,
      @Nullable byte[] rawData)
      throws IOException {
    if (isActive()) {
      observeStream(streamName, Arrays.asList(values), rawMetaData, maybeWrap(rawData));
    }
  }

  default <T> void observeStream(
      @NotNull String streamName,
      @NotNull Collection<T> values,
      @NotNull Iterable<ByteBuffer> rawMetaData,
      @Nullable byte[] rawData)
      throws IOException {
    if (isActive()) {
      observeStream(streamName, values, ByteBufferUtil.concat(rawMetaData), maybeWrap(rawData));
    }
  }

  default <T> void observeStream(
      @NotNull String streamName,
      @NotNull T[] values,
      @NotNull Iterable<ByteBuffer> rawMetaData,
      @Nullable byte[] rawData)
      throws IOException {
    if (isActive()) {
      observeStream(
          streamName,
          Arrays.asList(values),
          ByteBufferUtil.concat(rawMetaData),
          maybeWrap(rawData));
    }
  }

  default <T> void observeStream(
      @NotNull String streamName,
      @NotNull T[] values,
      @NotNull Iterable<ByteBuffer> rawMetaData,
      @NotNull ByteBuffer rawData)
      throws IOException {
    if (isActive()) {
      observeStream(streamName, Arrays.asList(values), ByteBufferUtil.concat(rawMetaData), rawData);
    }
  }

  default <T> void observeStream(
      @NotNull String streamName,
      @NotNull T[] values,
      @Nullable byte[] rawMetaData,
      @NotNull ByteBuffer rawData)
      throws IOException {
    if (isActive()) {
      observeStream(streamName, Arrays.asList(values), maybeWrap(rawMetaData), rawData);
    }
  }

  default void observeStream(
      @NotNull String streamName,
      @NotNull int[] values,
      @NotNull Iterable<ByteBuffer> rawMetaData,
      @NotNull ByteBuffer rawData)
      throws IOException {
    if (isActive()) {
      observeStream(streamName, IntStream.of(values).boxed().toArray(), rawMetaData, rawData);
    }
  }

  default void observeStream(
      @NotNull String streamName,
      @NotNull long[] values,
      @NotNull Iterable<ByteBuffer> rawMetaData,
      @NotNull ByteBuffer rawData)
      throws IOException {
    if (isActive()) {
      observeStream(streamName, LongStream.of(values).boxed().toArray(), rawMetaData, rawData);
    }
  }

  private static @Nullable ByteBuffer maybeWrap(@Nullable byte[] buffer) {
    return (buffer == null) ? emptyBuffer : ByteBuffer.wrap(buffer).asReadOnlyBuffer();
  }

  static final ByteBuffer emptyBuffer = ByteBuffer.wrap(new byte[0]).asReadOnlyBuffer();
}
