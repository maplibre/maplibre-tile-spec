package org.maplibre.mlt.metadata.stream;

public class LogicalStreamType {
  private DictionaryType dictionaryType;
  private OffsetType offsetType;
  private LengthType lengthType;

  public LogicalStreamType(DictionaryType dictionaryType) {
    this.dictionaryType = dictionaryType;
  }

  public LogicalStreamType(OffsetType offsetType) {
    this.offsetType = offsetType;
  }

  public LogicalStreamType(LengthType lengthType) {
    this.lengthType = lengthType;
  }

  public DictionaryType dictionaryType() {
    return this.dictionaryType;
  }

  public OffsetType offsetType() {
    return this.offsetType;
  }

  public LengthType lengthType() {
    return this.lengthType;
  }
}
