package org.maplibre.mlt.ffi;

/**
 * The size of one encoded layer.
 *
 * @param name the layer name
 * @param bytes the value of the layer's size varint: the tag and the body, without the varint itself
 */
public record MltLayerSize(String name, int bytes) {}
