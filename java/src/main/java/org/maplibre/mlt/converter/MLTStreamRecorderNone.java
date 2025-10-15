package org.maplibre.mlt.converter;

import java.util.Collection;

public class MLTStreamRecorderNone implements MLTStreamRecorder {
  public MLTStreamRecorderNone() {}

  @Override
  public void setLayerName(String layerName) {}

  @Override
  public boolean isActive() {
    return false;
  }

  @Override
  public <T> void recordStream(
      String streamName, Collection<T> values, byte[] rawMetaData, byte[] rawData) {}
}
