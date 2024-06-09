package com.mlt.converter.encodings;

import com.mlt.metadata.stream.*;
import org.apache.commons.lang3.ArrayUtils;

import java.util.List;

public class FloatEncoder {

    private FloatEncoder(){}

    public static byte[] encodeFloatStream(List<Float> values){
        //TODO: add encodings -> RLE, Dictionary, PDE, ALP
        float[] floatArray = new float[values.size()];
        for (int i = 0 ; i < values.size(); i++) {
            floatArray[i] = values.get(i);
        }
        var encodedValueStream = EncodingUtils.encodeFloatsLE(floatArray);

        var valuesMetadata = new StreamMetadata(PhysicalStreamType.DATA, null, LogicalLevelTechnique.NONE,
                LogicalLevelTechnique.NONE, PhysicalLevelTechnique.NONE, values.size(), encodedValueStream.length).encode();

        return ArrayUtils.addAll(valuesMetadata, encodedValueStream);
    }

}
