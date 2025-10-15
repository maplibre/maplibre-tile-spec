package org.maplibre.mlt.metadata.stream;

import com.google.common.primitives.Bytes;
import java.io.IOException;
import me.lemire.integercompression.IntWrapper;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.decoder.DecodingUtils;

public class StreamMetadata {
  private final PhysicalStreamType physicalStreamType;
  private final LogicalStreamType logicalStreamType;
  private final LogicalLevelTechnique logicalLevelTechnique1;
  private final LogicalLevelTechnique logicalLevelTechnique2;
  private final PhysicalLevelTechnique physicalLevelTechnique;
  /* After logical Level technique was applied -> when rle is used it is the length of the runs and values array */
  private final int numValues;
  private final int byteLength;

  // TODO: refactor -> use builder pattern
  public StreamMetadata(
      PhysicalStreamType physicalStreamType,
      LogicalStreamType logicalStreamType,
      LogicalLevelTechnique logicalLevelTechnique1,
      LogicalLevelTechnique logicalLevelTechnique2,
      PhysicalLevelTechnique physicalLevelTechnique,
      int numValues,
      int byteLength) {
    this.physicalStreamType = physicalStreamType;
    this.logicalStreamType = logicalStreamType;
    this.logicalLevelTechnique1 = logicalLevelTechnique1;
    this.logicalLevelTechnique2 = logicalLevelTechnique2;
    this.physicalLevelTechnique = physicalLevelTechnique;
    this.numValues = numValues;
    this.byteLength = byteLength;
  }

  private int getLogicalType() {
    if (logicalStreamType == null) {
      return 0;
    }

    if (logicalStreamType.dictionaryType() != null) {
      return logicalStreamType.dictionaryType().ordinal();
    }

    if (logicalStreamType.lengthType() != null) {
      return logicalStreamType.lengthType().ordinal();
    }

    return logicalStreamType.offsetType().ordinal();
  }

  public byte[] encode() throws IOException {
    final var encodedStreamType = (byte) ((physicalStreamType.ordinal()) << 4 | getLogicalType());
    final var encodedEncodingScheme =
        (byte)
            (logicalLevelTechnique1.ordinal() << 5
                | logicalLevelTechnique2.ordinal() << 2
                | physicalLevelTechnique.ordinal());
    final var encodedLengthInfo =
        EncodingUtils.encodeVarints(new int[] {numValues, byteLength}, false, false);
    return Bytes.concat(new byte[] {encodedStreamType, encodedEncodingScheme}, encodedLengthInfo);
  }

  public static StreamMetadata decode(byte[] tile, IntWrapper offset) throws IOException {
    var streamType = tile[offset.get()];
    var physicalStreamType = PhysicalStreamType.values()[streamType >> 4];
    LogicalStreamType logicalStreamType =
        switch (physicalStreamType) {
          case DATA -> new LogicalStreamType(DictionaryType.values()[streamType & 0xf]);
          case OFFSET -> new LogicalStreamType(OffsetType.values()[streamType & 0xf]);
          case LENGTH -> new LogicalStreamType(LengthType.values()[streamType & 0xf]);
          default -> null;
        };
    offset.increment();

    var encodingsHeader = tile[offset.get()] & 0xFF;
    var logicalLevelTechnique1 = LogicalLevelTechnique.values()[encodingsHeader >> 5];
    var logicalLevelTechnique2 = LogicalLevelTechnique.values()[encodingsHeader >> 2 & 0x7];
    var physicalLevelTechnique = PhysicalLevelTechnique.values()[encodingsHeader & 0x3];
    offset.increment();
    var sizeInfo = DecodingUtils.decodeVarints(tile, offset, 2);
    var numValues = sizeInfo[0];
    var byteLength = sizeInfo[1];

    return new StreamMetadata(
        physicalStreamType,
        logicalStreamType,
        logicalLevelTechnique1,
        logicalLevelTechnique2,
        physicalLevelTechnique,
        numValues,
        byteLength);
  }

  public PhysicalStreamType physicalStreamType() {
    return this.physicalStreamType;
  }

  public LogicalStreamType logicalStreamType() {
    return this.logicalStreamType;
  }

  public LogicalLevelTechnique logicalLevelTechnique1() {
    return this.logicalLevelTechnique1;
  }

  public LogicalLevelTechnique logicalLevelTechnique2() {
    return this.logicalLevelTechnique2;
  }

  public PhysicalLevelTechnique physicalLevelTechnique() {
    return this.physicalLevelTechnique;
  }

  public int numValues() {
    return this.numValues;
  }

  public int byteLength() {
    return this.byteLength;
  }
}
