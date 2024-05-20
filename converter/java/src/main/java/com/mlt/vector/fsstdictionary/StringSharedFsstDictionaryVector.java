package com.mlt.vector.fsstdictionary;

import com.mlt.vector.VariableSizeVector;
import com.mlt.vector.dictionary.DictionaryDataVector;

import java.nio.ByteBuffer;
import java.nio.IntBuffer;

public class StringSharedFsstDictionaryVector extends VariableSizeVector<String> {

    private final DictionaryDataVector[] dictionaryDataVectors;

    public StringSharedFsstDictionaryVector(String name, IntBuffer lengthBuffer, ByteBuffer dictionaryBuffer,
                                            IntBuffer symbolLengthBuffer,  ByteBuffer symbolTableBuffer,
                                            DictionaryDataVector[] dictionaryDataVectors) {
        super(name, lengthBuffer, dictionaryBuffer);
        this.dictionaryDataVectors = dictionaryDataVectors;
    }

    @Override
    protected String getValueFromBuffer(int index) {
        return "";
    }
}
