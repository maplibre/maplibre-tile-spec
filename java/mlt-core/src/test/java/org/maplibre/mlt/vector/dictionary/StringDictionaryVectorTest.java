package org.maplibre.mlt.vector.dictionary;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.common.collect.Lists;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.List;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.tuple.Pair;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.converter.MLTStreamObserverDefault;
import org.maplibre.mlt.converter.encodings.StringEncoder;
import org.maplibre.mlt.decoder.vectorized.VectorizedStringDecoder;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.vector.BitVector;

public class StringDictionaryVectorTest {
  public static Pair<Integer, byte[]> encode(
      List<String> values, PhysicalLevelTechnique physicalLevelTechnique, boolean useFsstEncoding)
      throws IOException {
    return StringEncoder.encode(
        values, physicalLevelTechnique, useFsstEncoding, new MLTStreamObserverDefault(), null);
  }

  @Test
  public void getValueFromBuffer() throws IOException {
    var values = List.of("Test", "Test1", "Test2", "Test4", "Test", "Test4", "Test1", "Test");
    var bitVector = new BitVector(ByteBuffer.wrap(new byte[] {(byte) 0xff}), 8);

    var encodedValues = encode(values, PhysicalLevelTechnique.FAST_PFOR, false);
    var vector =
        (StringDictionaryVector)
            VectorizedStringDecoder.decode(
                "test",
                encodedValues.getRight(),
                new IntWrapper(0),
                encodedValues.getLeft(),
                bitVector);

    for (var i = 0; i < values.size(); i++) {
      assertTrue(vector.getValue(i).isPresent());
      assertEquals(values.get(i), vector.getValue(i).get());
    }
  }

  @Test
  public void iterator() throws IOException {
    var values = List.of("Test", "Test1", "Test2", "Test4", "Test", "Test4", "Test1", "Test");
    var bitVector = new BitVector(ByteBuffer.wrap(new byte[] {(byte) 0xff}), 8);

    var encodedValues = encode(values, PhysicalLevelTechnique.FAST_PFOR, false);
    var vector =
        (StringDictionaryVector)
            VectorizedStringDecoder.decode(
                "test",
                encodedValues.getRight(),
                new IntWrapper(0),
                encodedValues.getLeft(),
                bitVector);

    var decodedValues = Lists.newArrayList(vector.iterator());
    assertEquals(values, decodedValues);
  }
}
