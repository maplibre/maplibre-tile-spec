package org.maplibre.mlt.vector.dictionary;

import org.maplibre.mlt.vector.BitVector;
import java.nio.IntBuffer;

public record DictionaryDataVector(
    String name, BitVector nullabilityBuffer, IntBuffer offsetBuffer) {}
