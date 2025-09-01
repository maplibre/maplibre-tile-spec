package org.maplibre.mlt.metadata.stream;

import com.google.common.primitives.Bytes;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.decoder.DecodingUtils;
import me.lemire.integercompression.IntWrapper;

public class StreamMetadata {
  private PhysicalStreamType physicalStreamType;
  private LogicalStreamType logicalStreamType;
  private LogicalLevelTechnique logicalLevelTechnique1;
  private LogicalLevelTechnique logicalLevelTechnique2;
  private PhysicalLevelTechnique physicalLevelTechnique;
  /* After logical Level technique was applied -> when rle is used it is the length of the runs and values array */
  private int numValues;
  private int byteLength;

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

  public byte[] encode() {
    var encodedStreamType = (byte) ((physicalStreamType.ordinal()) << 4 | getLogicalType());
    var encodedEncodingScheme =
        (byte)
            (logicalLevelTechnique1.ordinal() << 5
                | logicalLevelTechnique2.ordinal() << 2
                | physicalLevelTechnique.ordinal());
    var encodedLengthInfo =
        EncodingUtils.encodeVarints(new long[] {numValues, byteLength}, false, false);
    return Bytes.concat(new byte[] {encodedStreamType, encodedEncodingScheme}, encodedLengthInfo);
  }

  public static StreamMetadata decode(byte[] tile, IntWrapper offset) {
    var streamType = tile[offset.get()];
    var physicalStreamType = PhysicalStreamType.values()[streamType >> 4];
    LogicalStreamType logicalStreamType = null;
    switch (physicalStreamType) {
      case DATA:
        logicalStreamType = new LogicalStreamType(DictionaryType.values()[streamType & 0xf]);
        break;
      case OFFSET:
        logicalStreamType = new LogicalStreamType(OffsetType.values()[streamType & 0xf]);
        break;
      case LENGTH:
        logicalStreamType = new LogicalStreamType(LengthType.values()[streamType & 0xf]);
        break;
    }
    offset.increment();

    var encodingsHeader = tile[offset.get()] & 0xFF;
    var logicalLevelTechnique1 = LogicalLevelTechnique.values()[encodingsHeader >> 5];
    var logicalLevelTechnique2 = LogicalLevelTechnique.values()[encodingsHeader >> 2 & 0x7];
    var physicalLevelTechnique = PhysicalLevelTechnique.values()[encodingsHeader & 0x3];
    offset.increment();
    var sizeInfo = DecodingUtils.decodeVarint(tile, offset, 2);
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
