package com.mlt.vector.fsstdictionary;

import com.mlt.vector.BitVector;
import com.mlt.vector.VariableSizeVector;

import java.nio.ByteBuffer;
import java.nio.IntBuffer;

public class StringFsstDictionaryVector extends VariableSizeVector<String> {
    //TODO: extend from StringVector

    private IntBuffer offsets;
    private int[] offsetBuffer;
    private IntBuffer indexBuffer;

    public StringFsstDictionaryVector(String name, IntBuffer indexBuffer, IntBuffer lengthBuffer, ByteBuffer dictionaryBuffer,
                                     IntBuffer symbolLengthBuffer,  ByteBuffer symbolTableBuffer) {
        super(name, lengthBuffer, dictionaryBuffer);
        setBuffer(indexBuffer);
    }

    public StringFsstDictionaryVector(String name, BitVector nullabilityBuffer, IntBuffer indexBuffer, IntBuffer lengthBuffer,
                                  ByteBuffer dictionaryBuffer, IntBuffer symbolLengthBuffer,  ByteBuffer symbolTableBuffer) {
        super(name, nullabilityBuffer, lengthBuffer, dictionaryBuffer);
        setBuffer(indexBuffer);
    }

    private void setBuffer(IntBuffer indexBuffer){
        this.indexBuffer = indexBuffer;
        this.offsetBuffer = new int[indexBuffer.capacity()];
    }

    @Override
    protected String getValueFromBuffer(int index) {
        return "";
    }
}
