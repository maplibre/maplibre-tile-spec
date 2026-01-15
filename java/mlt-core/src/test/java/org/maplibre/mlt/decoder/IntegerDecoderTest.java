package org.maplibre.mlt.decoder;

import java.io.IOException;
import java.util.List;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.util.Assert;
import org.maplibre.mlt.converter.CollectionUtils;
import org.maplibre.mlt.converter.MLTStreamObserverDefault;
import org.maplibre.mlt.converter.encodings.*;
import org.maplibre.mlt.metadata.stream.*;

public class IntegerDecoderTest {
  @Test
  public void encode_Int_Limits() throws IOException {
    for (int v : new int[] {Integer.MIN_VALUE, -1, 0, 1, Integer.MAX_VALUE}) {
      final var encoded = EncodingUtils.encodeVarint(v, false);
      final var decoded = DecodingUtils.decodeVarints(encoded, new IntWrapper(0), 1)[0];
      Assert.equals(decoded, v);
    }
  }

  @Test
  public void encode_Int_Limits_ZigZag() throws IOException {
    for (int v : new int[] {Integer.MIN_VALUE, -1, 0, 1, Integer.MAX_VALUE}) {
      final var encoded = EncodingUtils.encodeVarint(v, true);
      final var zigzag = DecodingUtils.decodeVarints(encoded, new IntWrapper(0), 1)[0];
      final var decoded = DecodingUtils.decodeZigZag(zigzag);
      Assert.equals(decoded, v);
    }
  }

  private static byte[] encodeIntStream(
      List<Integer> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      @SuppressWarnings("SameParameterValue") boolean isSigned,
      @SuppressWarnings("SameParameterValue") PhysicalStreamType streamType,
      @SuppressWarnings("SameParameterValue") LogicalStreamType logicalStreamType)
      throws IOException {
    return IntegerEncoder.encodeIntStream(
        values,
        physicalLevelTechnique,
        isSigned,
        streamType,
        logicalStreamType,
        new MLTStreamObserverDefault(),
        null);
  }

  @Test
  public void decodeIntStream_SignedIntegerValues_PlainFastPforEncode() throws IOException {
    var values = List.of(1, 2, 7, 3, -4, 5, 1, -8);
    var encodedStream =
        encodeIntStream(
            values, PhysicalLevelTechnique.FAST_PFOR, true, PhysicalStreamType.DATA, null);

    var offset = new IntWrapper(0);
    var streamMetadata = StreamMetadata.decode(encodedStream, offset);
    var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

    Assert.equals(values, decodedValues);
    Assert.equals(LogicalLevelTechnique.NONE, streamMetadata.logicalLevelTechnique1());
  }

  @Test
  public void decodeIntStream_SignedIntegerValues_PlainVarintEncode() throws IOException {
    var values = List.of(1, 2, 7, 3, -4, 5, 1, -8);
    var encodedStream =
        encodeIntStream(values, PhysicalLevelTechnique.VARINT, true, PhysicalStreamType.DATA, null);

    var offset = new IntWrapper(0);
    var streamMetadata = StreamMetadata.decode(encodedStream, offset);
    var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

    Assert.equals(values, decodedValues);
    Assert.equals(LogicalLevelTechnique.NONE, streamMetadata.logicalLevelTechnique1());
  }

  @Test
  @Disabled
  public void decodeIntStream_SignedIntegerValues_FastPforDeltaRleEncode() throws IOException {
    var values = List.of(-1, -2, -3, -4, -5, -6, -7, 8);
    var encodedStream =
        encodeIntStream(
            values, PhysicalLevelTechnique.FAST_PFOR, true, PhysicalStreamType.DATA, null);

    var offset = new IntWrapper(0);
    var streamMetadata = StreamMetadata.decode(encodedStream, offset);
    var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

    Assert.equals(values, decodedValues);
  }

  @Test
  @Disabled
  public void decodeIntStream_SignedIntegerValues_VarintDeltaRleEncode() throws IOException {
    var values = List.of(-1, -2, -3, -4, -5, -6, -7, 8);
    var encodedStream =
        encodeIntStream(values, PhysicalLevelTechnique.VARINT, true, PhysicalStreamType.DATA, null);

    var offset = new IntWrapper(0);
    var streamMetadata = StreamMetadata.decode(encodedStream, offset);
    var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

    Assert.equals(values, decodedValues);
  }

  @Test
  @Disabled
  public void decodeIntStream_SignedIntegerValues_FastPforRleEncode() throws IOException {
    var values = List.of(-1, -1, -1, -1, -1, -1, -2, -2);
    var encodedStream =
        encodeIntStream(
            values, PhysicalLevelTechnique.FAST_PFOR, true, PhysicalStreamType.DATA, null);

    var offset = new IntWrapper(0);
    var streamMetadata = StreamMetadata.decode(encodedStream, offset);
    var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

    Assert.equals(values, decodedValues);
    Assert.equals(LogicalLevelTechnique.RLE, streamMetadata.logicalLevelTechnique1());
  }

  @Test
  @Disabled
  public void decodeIntStream_SignedIntegerValues_VarintRleEncode() throws IOException {
    var values = List.of(-1, -1, -1, -1, -1, -1, -2, -2);
    var encodedStream =
        encodeIntStream(values, PhysicalLevelTechnique.VARINT, true, PhysicalStreamType.DATA, null);

    var offset = new IntWrapper(0);
    var streamMetadata = StreamMetadata.decode(encodedStream, offset);
    var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

    Assert.equals(values, decodedValues);
    Assert.equals(LogicalLevelTechnique.RLE, streamMetadata.logicalLevelTechnique1());
  }

  @Test
  @Disabled
  public void decodeIntStream_UnsignedIntegerValues_VarintRleEncode() throws IOException {
    var values = List.of(1, 1, 1, 1, 1, 1, 2, 2);
    var encodedStream =
        encodeIntStream(values, PhysicalLevelTechnique.VARINT, true, PhysicalStreamType.DATA, null);

    var offset = new IntWrapper(0);
    var streamMetadata = StreamMetadata.decode(encodedStream, offset);
    var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

    Assert.equals(values, decodedValues);
    Assert.equals(LogicalLevelTechnique.RLE, streamMetadata.logicalLevelTechnique1());
  }

  private static byte[] encodeLongStream(
      List<Long> values,
      @SuppressWarnings("SameParameterValue") boolean isSigned,
      @SuppressWarnings("SameParameterValue") PhysicalStreamType streamType,
      @SuppressWarnings("SameParameterValue") LogicalStreamType logicalStreamType)
      throws IOException {
    return IntegerEncoder.encodeLongStream(
        CollectionUtils.unboxLongs(values),
        isSigned,
        streamType,
        logicalStreamType,
        new MLTStreamObserverDefault(),
        null);
  }

  @Test
  @Disabled
  public void decodeLongStream_SignedIntegerValues_PlainEncode() throws IOException {
    final var values = List.of(1L, 2L, 7L, 3L, -4L, 5L, 1L, -8L);
    var encodedStream = encodeLongStream(values, true, PhysicalStreamType.DATA, null);

    var offset = new IntWrapper(0);
    var streamMetadata = StreamMetadata.decode(encodedStream, offset);
    var decodedValues =
        IntegerDecoder.decodeLongStream(encodedStream, offset, streamMetadata, true);

    Assert.equals(values, decodedValues);
    Assert.equals(LogicalLevelTechnique.NONE, streamMetadata.logicalLevelTechnique1());
  }

  @Test
  @Disabled
  public void decodeLongStream_SignedIntegerValues_DeltaRleEncode() throws IOException {
    final var values = List.of(-1L, -2L, -3L, -4L, -5L, -6L, -7L, 8L);
    var encodedStream = encodeLongStream(values, true, PhysicalStreamType.DATA, null);

    var offset = new IntWrapper(0);
    var streamMetadata = StreamMetadata.decode(encodedStream, offset);
    var decodedValues =
        IntegerDecoder.decodeLongStream(encodedStream, offset, streamMetadata, true);

    Assert.equals(values, decodedValues);
  }

  @Test
  @Disabled
  public void decodeLongStream_SignedIntegerValues_RleEncode() throws IOException {
    final var values = List.of(-1L, -1L, -1L, -1L, -1L, -1L, -2L, -2L);
    var encodedStream = encodeLongStream(values, true, PhysicalStreamType.DATA, null);

    var offset = new IntWrapper(0);
    var streamMetadata = StreamMetadata.decode(encodedStream, offset);
    var decodedValues =
        IntegerDecoder.decodeLongStream(encodedStream, offset, streamMetadata, true);

    Assert.equals(values, decodedValues);
    Assert.equals(LogicalLevelTechnique.RLE, streamMetadata.logicalLevelTechnique1());
  }
}
