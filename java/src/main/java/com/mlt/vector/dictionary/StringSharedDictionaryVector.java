package com.mlt.vector.dictionary;

import com.mlt.vector.VariableSizeVector;
import me.lemire.integercompression.differential.Delta;
import org.apache.commons.lang3.ArrayUtils;

import java.nio.ByteBuffer;
import java.nio.IntBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashMap;
import java.util.Map;

public class StringSharedDictionaryVector extends VariableSizeVector<Map<String, String>> {
    /** Specifies where a specific string starts in the data buffer for a given index */
    private int[] dictionaryOffsetBuffer;
    private final DictionaryDataVector[] dictionaryDataVectors;
    private Map<String, DictionaryDataVector> decodedDictionaryDataVectors;

    public StringSharedDictionaryVector(String name, IntBuffer lengthBuffer, ByteBuffer dictionaryBuffer,
                                        DictionaryDataVector[] dictionaryDataVectors) {
        super(name, lengthBuffer, dictionaryBuffer);
        this.dictionaryDataVectors = dictionaryDataVectors;
    }

    public static  StringSharedDictionaryVector createFromOffsetBuffer(String name, IntBuffer offsetBuffer,
                                                                       ByteBuffer dictionaryBuffer,
                                                                       DictionaryDataVector[] dictionaryDataVectors) {
        var vector = new StringSharedDictionaryVector(name, null, dictionaryBuffer, dictionaryDataVectors);
        vector.dictionaryOffsetBuffer = offsetBuffer.array();
        return vector;
    }

    @Override
    protected Map<String, String> getValueFromBuffer(int index) {
        var values = new HashMap<String, String>();
        Arrays.stream(dictionaryDataVectors).forEach(vector -> values.put(vector.name(),
                getPresentValue(vector.name(), index)));
        return values;
    }

    public String getValue(String name, int index){
        if(nullabilityBuffer.isPresent() && !nullabilityBuffer.get().get(index)){
            return null;
        }

        return getPresentValue(name, index);
    }

    private String getPresentValue(String name, int index){
        if (dictionaryOffsetBuffer == null){
            decodeLengthBuffer();
        }

        if(decodedDictionaryDataVectors == null){
            createDataVectorsMap();
        }

        var dataVector = decodedDictionaryDataVectors.get(name);
        if(!dataVector.nullabilityBuffer().get(index)){
            return null;
        }

        var indexBuffer = dataVector.offsetBuffer();
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
