package org.maplibre.mlt.vector.dictionary;

import java.nio.IntBuffer;
import org.maplibre.mlt.vector.BitVector;

public record DictionaryDataVector(
    String name, BitVector nullabilityBuffer, IntBuffer offsetBuffer) {}
