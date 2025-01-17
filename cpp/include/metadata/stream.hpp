#pragma once

#include <common.hpp>
#include <decoding_utils.hpp>

#include <optional>
#include <memory>

namespace mlt::metadata::stream {

enum class DictionaryType {
    NONE = 0,
    SINGLE = 1,
    SHARED = 2,
    VERTEX = 3,
    MORTON = 4,
    FSST = 5,
};

enum class LengthType {
    VAR_BINARY = 0,
    GEOMETRIES = 1,
    PARTS = 2,
    RINGS = 3,
    TRIANGLES = 4,
    SYMBOL = 5,
    DICTIONARY = 6,
};

enum class PhysicalLevelTechnique {
    NONE = 0,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    /// But currently limited to 32-bit integer.
    FAST_PFOR = 1,
    /// Can produce better results in combination with a heavyweight compression scheme like Gzip.
    /// Simple compression scheme where the decoder are easier to implement compared to FastPfor.
    VARINT = 2,
    /// Adaptive Lossless floating-Point Compression
    ALP = 3,
};

enum class LogicalLevelTechnique {
    NONE = 0,
    DELTA = 1,
    COMPONENTWISE_DELTA = 2,
    RLE = 3,
    MORTON = 4,
    PSEUDODECIMAL = 5,
};

enum class OffsetType {
    VERTEX = 0,
    INDEX = 1,
    STRING = 2,
    KEY = 3,
};

enum class PhysicalStreamType {
    PRESENT = 0,
    DATA = 1,
    OFFSET = 2,
    LENGTH = 3,
};

class LogicalStreamType {
public:
    LogicalStreamType(DictionaryType type)
        : dictionaryType(type) {}
    LogicalStreamType(OffsetType type)
        : offsetType(type) {}
    LogicalStreamType(LengthType type)
        : lengthType(type) {}

    LogicalStreamType() = delete;
    LogicalStreamType(const LogicalStreamType&) = delete;
    LogicalStreamType(LogicalStreamType&&) = default;

    const std::optional<DictionaryType>& getDictionaryType() const { return dictionaryType; }
    const std::optional<OffsetType>& getOffsetType() const { return offsetType; }
    const std::optional<LengthType>& getLengthType() const { return lengthType; }

private:
    std::optional<DictionaryType> dictionaryType;
    std::optional<OffsetType> offsetType;
    std::optional<LengthType> lengthType;
};

class StreamMetadata;
std::unique_ptr<StreamMetadata> decode(DataView, offset_t&);

class StreamMetadata {
public:
    StreamMetadata(PhysicalStreamType physicalStreamType_,
                   std::optional<LogicalStreamType> logicalStreamType_,
                   LogicalLevelTechnique logicalLevelTechnique1_,
                   LogicalLevelTechnique logicalLevelTechnique2_,
                   PhysicalLevelTechnique physicalLevelTechnique_,
                   int numValues_,
                   int byteLength_)
        : physicalStreamType(physicalStreamType_),
          logicalStreamType(std::move(logicalStreamType_)),
          logicalLevelTechnique1(logicalLevelTechnique1_),
          logicalLevelTechnique2(logicalLevelTechnique2_),
          physicalLevelTechnique(physicalLevelTechnique_),
          numValues(numValues_),
          byteLength(byteLength_) {}

private:
    int getLogicalType();

    static StreamMetadata decode(DataView tileData, offset_t& offset);

    PhysicalStreamType physicalStreamType;
    std::optional<LogicalStreamType> logicalStreamType;
    LogicalLevelTechnique logicalLevelTechnique1;
    LogicalLevelTechnique logicalLevelTechnique2;
    PhysicalLevelTechnique physicalLevelTechnique;

    // After logical Level technique was applied -> when rle is used it is the length of the runs and values array
    int numValues;
    int byteLength;
};

#if 0

class RleEncodedStreamMetadata : public StreamMetadata {
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
};

class MortonEncodedStreamMetadata extends StreamMetadata {
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
};

#endif

class PdeEncodedMetadata {};

} // namespace mlt::metadata::stream
