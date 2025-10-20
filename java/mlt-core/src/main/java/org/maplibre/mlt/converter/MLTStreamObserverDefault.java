package org.maplibre.mlt.converter;

import java.util.Collection;

/** A no-operation implementation of the MLTStreamObserver interface. */
public class MLTStreamObserverDefault implements MLTStreamObserver {
  public MLTStreamObserverDefault() {}

  @Override
  public void setLayerName(String layerName) {}

  @Override
  public boolean isActive() {
    return false;
  }

  @Override
  public <T> void observeStream(
      String streamName, Collection<T> values, byte[] rawMetaData, byte[] rawData) {}
}
