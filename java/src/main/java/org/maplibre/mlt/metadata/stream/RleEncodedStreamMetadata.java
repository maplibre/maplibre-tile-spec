package org.maplibre.mlt.metadata.stream;

import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.ArrayUtils;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.decoder.DecodingUtils;

public class RleEncodedStreamMetadata extends StreamMetadata {
  int runs;
  int numRleValues;

  // TODO: refactor -> use builder pattern

  /**
   * Only used for RLE encoded integer values. Not needed for rle encoded boolean and byte values.
   *
   * @param numValues After LogicalLevelTechnique was applied -> numRuns + numValues
   * @param runs Length of the runs array
   * @param numRleValues Used for pre-allocating the arrays on the client for faster decoding
   */
  public RleEncodedStreamMetadata(
      PhysicalStreamType physicalStreamType,
      LogicalStreamType logicalStreamType,
      LogicalLevelTechnique logicalLevelTechnique1,
      LogicalLevelTechnique logicalLevelTechnique2,
      PhysicalLevelTechnique physicalLevelTechnique,
      int numValues,
      int byteLength,
      int runs,
      int numRleValues) {
    super(
        physicalStreamType,
        logicalStreamType,
        logicalLevelTechnique1,
        logicalLevelTechnique2,
        physicalLevelTechnique,
        numValues,
        byteLength);
    this.runs = runs;
    this.numRleValues = numRleValues;
  }

  public byte[] encode() {
    var encodedRleInfo = EncodingUtils.encodeVarints(new long[] {runs, numRleValues}, false, false);
    return ArrayUtils.addAll(super.encode(), encodedRleInfo);
  }

  public static RleEncodedStreamMetadata decode(byte[] tile, IntWrapper offset) {
    var streamMetadata = StreamMetadata.decode(tile, offset);
    var rleInfo = DecodingUtils.decodeVarint(tile, offset, 2);
    return new RleEncodedStreamMetadata(
        streamMetadata.physicalStreamType(),
        streamMetadata.logicalStreamType(),
        streamMetadata.logicalLevelTechnique1(),
        streamMetadata.logicalLevelTechnique2(),
        streamMetadata.physicalLevelTechnique(),
        streamMetadata.numValues(),
        streamMetadata.byteLength(),
        rleInfo[0],
        rleInfo[1]);
  }

  public static RleEncodedStreamMetadata decodePartial(
      StreamMetadata streamMetadata, byte[] tile, IntWrapper offset) {
    var rleInfo = DecodingUtils.decodeVarint(tile, offset, 2);
    return new RleEncodedStreamMetadata(
        streamMetadata.physicalStreamType(),
        streamMetadata.logicalStreamType(),
        streamMetadata.logicalLevelTechnique1(),
        streamMetadata.logicalLevelTechnique2(),
        streamMetadata.physicalLevelTechnique(),
        streamMetadata.numValues(),
        streamMetadata.byteLength(),
        rleInfo[0],
        rleInfo[1]);
  }

  public int runs() {
    return this.runs;
  }

  public int numRleValues() {
    return this.numRleValues;
  }
}
