package org.maplibre.mlt.converter;

import java.io.IOException;
import java.util.Collection;

public interface MLTStreamRecorder {

  boolean isActive();

  void setLayerName(String layerName);

  <T> void recordStream(String streamName, Collection<T> values, byte[] rawMetaData, byte[] rawData)
      throws IOException;
}
