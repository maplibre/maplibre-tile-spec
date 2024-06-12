package com.mlt.vector.dictionary;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.common.collect.Lists;
import com.mlt.converter.encodings.StringEncoder;
import com.mlt.decoder.vectorized.VectorizedStringDecoder;
import com.mlt.metadata.stream.PhysicalLevelTechnique;
import java.io.IOException;
import java.util.List;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Test;

public class StringDictionaryVectorTest {

  @Test
  public void getValueFromBuffer() throws IOException {
    var values =
        List.of("Test", "Test1", "Test2", "Test4", "Test", "Test4", "Test1", "Test", "Test4");

    var encodedValues = StringEncoder.encode(values, PhysicalLevelTechnique.FAST_PFOR, false);
    var vector =
        (StringDictionaryVector)
            VectorizedStringDecoder.decode(
                "test", encodedValues.getRight(), new IntWrapper(0), encodedValues.getLeft(), null);

    for (var i = 0; i < values.size(); i++) {
      assertEquals(values.get(i), vector.getValue(i).get());
    }
  }

  @Test
  public void iterator() throws IOException {
    var values =
        List.of("Test", "Test1", "Test2", "Test4", "Test", "Test4", "Test1", "Test", "Test4");

    var encodedValues = StringEncoder.encode(values, PhysicalLevelTechnique.FAST_PFOR, false);
    var vector =
        (StringDictionaryVector)
            VectorizedStringDecoder.decode(
                "test", encodedValues.getRight(), new IntWrapper(0), encodedValues.getLeft(), null);

    var decodedValues = Lists.newArrayList(vector.iterator());
    assertTrue(values.equals(decodedValues));
  }
}
