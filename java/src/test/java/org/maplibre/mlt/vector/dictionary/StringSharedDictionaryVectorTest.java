package org.maplibre.mlt.vector.dictionary;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.IOException;
import java.util.List;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.tuple.Pair;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.converter.MLTStreamRecorderNone;
import org.maplibre.mlt.converter.encodings.StringEncoder;
import org.maplibre.mlt.decoder.vectorized.VectorizedStringDecoder;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class StringSharedDictionaryVectorTest {

  public static Pair<Integer, byte[]> encodeSharedDictionary(
      List<List<String>> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useFsstEncoding)
      throws IOException {
    return StringEncoder.encodeSharedDictionary(
        values, physicalLevelTechnique, useFsstEncoding, new MLTStreamRecorderNone(), null);
  }

  @Test
  public void decodeSharedDictionary() throws IOException {
    var dict1 = List.of("Test", "Test1", "Test2", "Test3");
    var dict2 = List.of("Test4", "Test5", "Test6", "Test7");
    var sharedDict = List.of(dict1, dict2);
    var encodedDictionary =
        encodeSharedDictionary(sharedDict, PhysicalLevelTechnique.FAST_PFOR, false);
    var test =
        MltTilesetMetadata.Field.newBuilder()
            .setName("Test")
            .setScalarField(
                MltTilesetMetadata.ScalarField.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ScalarType.STRING)
                    .build());
    var test2 =
        MltTilesetMetadata.Field.newBuilder()
            .setName("Test2")
            .setScalarField(
                MltTilesetMetadata.ScalarField.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ScalarType.STRING)
                    .build());
    var column =
        MltTilesetMetadata.Column.newBuilder()
            .setName("Parent")
            .setNullable(true)
            .setComplexType(
                MltTilesetMetadata.ComplexColumn.newBuilder()
                    .addChildren(test)
                    .addChildren(test2)
                    .build())
            .build();

    var vector =
        (StringSharedDictionaryVector)
            VectorizedStringDecoder.decodeSharedDictionary(
                encodedDictionary.getRight(), new IntWrapper(0), column);

    for (var i = 0; i < dict1.size(); i++) {
      var value = vector.getValue("Parent:Test", i);
      assertEquals(dict1.get(i), value);
      var value2 = vector.getValue("Parent:Test2", i);
      assertEquals(dict2.get(i), value2);
    }
  }
}
