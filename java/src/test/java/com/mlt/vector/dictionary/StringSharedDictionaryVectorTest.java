package com.mlt.vector.dictionary;

import com.mlt.converter.encodings.StringEncoder;
import com.mlt.decoder.vectorized.VectorizedStringDecoder;
import com.mlt.metadata.stream.PhysicalLevelTechnique;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;

public class StringSharedDictionaryVectorTest {


    @Test
    public void decodeSharedDictionary() throws IOException {
        var dict1 = List.of("Test", "Test1", "Test2", "Test3");
        var dict2 = List.of("Test4", "Test5", "Test6", "Test7");
        var sharedDict = List.of(dict1, dict2);
        var encodedDictionary = StringEncoder.encodeSharedDictionary(sharedDict, PhysicalLevelTechnique.FAST_PFOR);
        var test = MltTilesetMetadata.Field.newBuilder().setName("Test").setScalarField(
                MltTilesetMetadata.ScalarField.newBuilder().setPhysicalType(MltTilesetMetadata.ScalarType.STRING).build());
        var test2 = MltTilesetMetadata.Field.newBuilder().setName("Test2").setScalarField(
                MltTilesetMetadata.ScalarField.newBuilder().setPhysicalType(MltTilesetMetadata.ScalarType.STRING).build());
        var column = MltTilesetMetadata.Column.newBuilder().setName("Parent").setNullable(true).setComplexType(
                MltTilesetMetadata.ComplexColumn.newBuilder().addChildren(test).addChildren(test2).build()).build();

        var vector = (StringSharedDictionaryVector)VectorizedStringDecoder.decodeSharedDictionary(encodedDictionary.getRight(), new IntWrapper(0), column);

        for(var i = 0; i < dict1.size(); i++){
            var value = vector.getValue("Parent:Test", i);
            assertEquals(dict1.get(i), value);
            var value2 = vector.getValue("Parent:Test2", i);
            assertEquals(dict2.get(i), value2);
        }
    }

}
