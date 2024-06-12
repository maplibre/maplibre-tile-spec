package com.mlt.metadata.stream;

import com.mlt.converter.encodings.EncodingUtils;
import com.mlt.decoder.DecodingUtils;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.ArrayUtils;

public class MortonEncodedStreamMetadata extends StreamMetadata {
  private int numBits;
  private int coordinateShift;

  // TODO: refactor -> use builder pattern
  public MortonEncodedStreamMetadata(
      PhysicalStreamType physicalStreamType,
      LogicalStreamType logicalStreamType,
      LogicalLevelTechnique logicalLevelTechnique1,
      LogicalLevelTechnique logicalLevelTechnique2,
      PhysicalLevelTechnique physicalLevelTechnique,
      int numValues,
      int byteLength,
      int numBits,
      int coordinateShift) {
    super(
        physicalStreamType,
        logicalStreamType,
        logicalLevelTechnique1,
        logicalLevelTechnique2,
        physicalLevelTechnique,
        numValues,
        byteLength);
    this.numBits = numBits;
    this.coordinateShift = coordinateShift;
  }

  public byte[] encode() {
    var mortonInfos =
        EncodingUtils.encodeVarints(new long[] {numBits, coordinateShift}, false, false);
    return ArrayUtils.addAll(super.encode(), mortonInfos);
  }

  public static MortonEncodedStreamMetadata decode(byte[] tile, IntWrapper offset) {
    var streamMetadata = StreamMetadata.decode(tile, offset);
    var mortonInfo = DecodingUtils.decodeVarint(tile, offset, 2);
    return new MortonEncodedStreamMetadata(
        streamMetadata.physicalStreamType(),
        streamMetadata.logicalStreamType(),
        streamMetadata.logicalLevelTechnique1(),
        streamMetadata.logicalLevelTechnique2(),
        streamMetadata.physicalLevelTechnique(),
        streamMetadata.numValues(),
        streamMetadata.byteLength(),
        mortonInfo[0],
        mortonInfo[1]);
  }

  public static MortonEncodedStreamMetadata decodePartial(
      StreamMetadata streamMetadata, byte[] tile, IntWrapper offset) {
    var mortonInfo = DecodingUtils.decodeVarint(tile, offset, 2);
    return new MortonEncodedStreamMetadata(
        streamMetadata.physicalStreamType(),
        streamMetadata.logicalStreamType(),
        streamMetadata.logicalLevelTechnique1(),
        streamMetadata.logicalLevelTechnique2(),
        streamMetadata.physicalLevelTechnique(),
        streamMetadata.numValues(),
        streamMetadata.byteLength(),
        mortonInfo[0],
        mortonInfo[1]);
  }

  public int numBits() {
    return this.numBits;
  }

  public int coordinateShift() {
    return this.coordinateShift;
  }
}
