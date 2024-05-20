package com.mlt.converter;

import org.apache.commons.lang3.ArrayUtils;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

public class CollectionUtils {
    private CollectionUtils(){}

    public static Optional<List<Integer>> toIntList(List<Long> values){
        var convertedValues = new ArrayList<Integer>();
        for(var id : values){
            if(id <= Integer.MAX_VALUE){
                convertedValues.add(id.intValue());
            }
            else{
                return Optional.empty();
            }
        }

        return Optional.of(convertedValues);
    }

    public static byte[] concatByteArrays(byte[]... arrays){
        var concatenatedArray = new byte[0];
        for(var array : arrays){
            concatenatedArray = ArrayUtils.addAll(concatenatedArray, array);
        }
        return concatenatedArray;
    }
}
