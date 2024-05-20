package com.mlt.vector.dictionary;

import com.mlt.vector.VariableSizeVector;
import me.lemire.integercompression.differential.Delta;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.commons.lang3.NotImplementedException;

import javax.naming.OperationNotSupportedException;
import java.nio.ByteBuffer;
import java.nio.IntBuffer;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;

public class StringSharedDictionaryVector extends VariableSizeVector<String> {
    /** Specifies where a specific string starts in the data buffer for a given index */
    private int[] dictionaryOffsetBuffer;
    //private int[] lazyOffsetBuffer;
    private final DictionaryDataVector[] dictionaryDataVectors;
    private Map<String, DictionaryDataVector> decodedDictionaryDataVectors;

    public StringSharedDictionaryVector(String name, IntBuffer lengthBuffer, ByteBuffer dictionaryBuffer,
                                        DictionaryDataVector[] dictionaryDataVectors) {
        super(name, lengthBuffer, dictionaryBuffer);
        this.dictionaryDataVectors = dictionaryDataVectors;
    }

    @Override
    protected String getValueFromBuffer(int index) {
        throw new UnsupportedOperationException("Method not supported for shared dictionary vectors");
    }

    public String getValue(String name, int index){
        if(nullabilityBuffer.isPresent() && !nullabilityBuffer.get().get(index)){
            return null;
        }

        if (dictionaryOffsetBuffer  == null){
            decodeLengthBuffer();
            createDataVectorsMap();
        }

        var indexBuffer = decodedDictionaryDataVectors.get(name).offsetBuffer();
        var offset = indexBuffer.get(index);
        var start = dictionaryOffsetBuffer[offset];
        var length = dictionaryOffsetBuffer[offset + 1] - start;
        var strBuffer = dataBuffer.slice(dataBuffer.position() + start, length);
        return StandardCharsets.UTF_8.decode(strBuffer).toString();
    }

    private void createDataVectorsMap(){
        decodedDictionaryDataVectors = new HashMap<>(dictionaryDataVectors.length);
        for(var dictionaryDataVector : dictionaryDataVectors){
            decodedDictionaryDataVectors.put(dictionaryDataVector.name(), dictionaryDataVector);
        }
    }

    private void decodeLengthBuffer(){
        //TODO: get rid of the array copy
        dictionaryOffsetBuffer = ArrayUtils.addAll(new int[]{0}, lengthBuffer.array());
        Delta.fastinverseDelta(dictionaryOffsetBuffer, 0, dictionaryOffsetBuffer.length, 0);
    }

}
