package com.mlt.vector.dictionary;

import com.mlt.vector.BitVector;

import java.nio.IntBuffer;

public record DictionaryDataVector(String name, BitVector nullabilityBuffer, IntBuffer offsetBuffer){ }