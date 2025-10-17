package org.maplibre.mlt.converter;

import java.io.IOException;
import java.util.Collection;

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
      String streamName, Collection<T> values, byte[] rawMetaData, byte[] rawData)
      throws IOException;
}
