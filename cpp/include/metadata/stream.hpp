#pragma once

#include <common.hpp>
#include <util/buffer_stream.hpp>
#include <util/varint.hpp>

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
std::unique_ptr<StreamMetadata> decode(BufferStream&);

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

    StreamMetadata() = delete;
    StreamMetadata(const StreamMetadata&) = delete;
    StreamMetadata(StreamMetadata&&) = default;

    PhysicalStreamType getPhysicalStreamType() const { return physicalStreamType; }
    const std::optional<LogicalStreamType>& getLogicalStreamType() const { return logicalStreamType; }
    LogicalLevelTechnique getLogicalLevelTechnique1() const { return logicalLevelTechnique1; }
    LogicalLevelTechnique getLogicalLevelTechnique2() const { return logicalLevelTechnique2; }
    PhysicalLevelTechnique getPhysicalLevelTechnique() const { return physicalLevelTechnique; }

    int getNumValues() const { return numValues; }
    int getByteLength() const { return byteLength; }

private:
    int getLogicalType();

    friend class RleEncodedStreamMetadata;
    friend class MortonEncodedStreamMetadata;
    friend std::unique_ptr<StreamMetadata> decode(BufferStream&);
    static StreamMetadata decode(BufferStream&);

    PhysicalStreamType physicalStreamType;
    std::optional<LogicalStreamType> logicalStreamType;
    LogicalLevelTechnique logicalLevelTechnique1;
    LogicalLevelTechnique logicalLevelTechnique2;
    PhysicalLevelTechnique physicalLevelTechnique;

    // After logical Level technique was applied -> when rle is used it is the length of the runs and values array
    int numValues;
    int byteLength;
};

class RleEncodedStreamMetadata : public StreamMetadata {
public:
    /**
     * Only used for RLE encoded integer values, not boolean and byte values.
     *
     * @param numValues After LogicalLevelTechnique was applied -> numRuns + numValues
     * @param runs Length of the runs array
     * @param numRleValues Used for pre-allocating the arrays on the client for faster decoding
     */
    RleEncodedStreamMetadata(
        PhysicalStreamType physicalStreamType_,
        std::optional<LogicalStreamType> logicalStreamType_,
        LogicalLevelTechnique logicalLevelTechnique1_,
        LogicalLevelTechnique logicalLevelTechnique2_,
        PhysicalLevelTechnique physicalLevelTechnique_,
        int numValues_,
        int byteLength_,
        int runs_,
        int numRleValues_) :
      StreamMetadata(
          physicalStreamType_,
          std::move(logicalStreamType_),
          logicalLevelTechnique1_,
          logicalLevelTechnique2_,
          physicalLevelTechnique_,
          numValues_,
          byteLength_),
      runs(runs_),
      numRleValues(numRleValues_) {
    }

    RleEncodedStreamMetadata(StreamMetadata&& streamMetadata, int runs_, int numRleValues_) :
      StreamMetadata(std::move(streamMetadata)),
      runs(runs_),
      numRleValues(numRleValues_) {
    }

    RleEncodedStreamMetadata() = delete;
    RleEncodedStreamMetadata(const RleEncodedStreamMetadata&) = delete;
    RleEncodedStreamMetadata(RleEncodedStreamMetadata&&) = default;

    static RleEncodedStreamMetadata decodePartial(StreamMetadata&& streamMetadata, BufferStream& buffer) {
      const auto [runs, numValues] = util::decoding::decodeVarints<2>(buffer);
      return RleEncodedStreamMetadata(std::move(streamMetadata), runs, numValues);
    }

    static RleEncodedStreamMetadata decode(BufferStream& buffer) {
      return decodePartial(StreamMetadata::decode(buffer), buffer);
    }

    int getRuns() const { return runs; }
    int getNumRleValues() const { return numRleValues; }

private:
    int runs;
    int numRleValues;
};

class MortonEncodedStreamMetadata : public StreamMetadata {
public:
    MortonEncodedStreamMetadata(
        PhysicalStreamType physicalStreamType_,
        LogicalStreamType logicalStreamType_,
        LogicalLevelTechnique logicalLevelTechnique1_,
        LogicalLevelTechnique logicalLevelTechnique2_,
        PhysicalLevelTechnique physicalLevelTechnique_,
        int numValues_,
        int byteLength_,
        int numBits_,
        int coordinateShift_) :
      StreamMetadata(
          physicalStreamType_,
          std::move(logicalStreamType_),
          logicalLevelTechnique1_,
          logicalLevelTechnique2_,
          physicalLevelTechnique_,
          numValues_,
          byteLength_),
      numBits(numBits_),
      coordinateShift(coordinateShift_) {
    }

    MortonEncodedStreamMetadata(
        StreamMetadata&& streamMetadata,
        int numBits_,
        int coordinateShift_) :
      StreamMetadata(std::move(streamMetadata)),
      numBits(numBits_),
      coordinateShift(coordinateShift_) {
    }

    static MortonEncodedStreamMetadata decodePartial(StreamMetadata&& streamMetadata, BufferStream& buffer) {
      const auto [numBits, coordShift] = util::decoding::decodeVarints<2>(buffer);
      return MortonEncodedStreamMetadata(std::move(streamMetadata), numBits, coordShift);
    }

    static MortonEncodedStreamMetadata decode(BufferStream& buffer) {
      auto streamMetadata = StreamMetadata::decode(buffer);
      return decodePartial(std::move(streamMetadata), buffer);
    }

    int getNumBits() const { return numBits; }
    int getCoordinateShift() const { return coordinateShift; }

private:
    int numBits;
    int coordinateShift;
};

class PdeEncodedMetadata {};

} // namespace mlt::metadata::stream
