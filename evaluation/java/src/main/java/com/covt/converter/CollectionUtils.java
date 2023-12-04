package com.covt.converter;

import org.apache.commons.lang3.ArrayUtils;

import java.util.List;

public class CollectionUtils {

    public static byte[] concatByteArrays(List<byte[]> values){
        var buffer = new byte[0];
        for(var value : values){
            buffer = ArrayUtils.addAll(buffer, value);
        }
        return buffer;
    }

}
