#pragma once

#include <mlt/common.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/varint.hpp>

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
    LogicalStreamType(DictionaryType type) noexcept
        : dictionaryType(type) {}
    LogicalStreamType(OffsetType type) noexcept
        : offsetType(type) {}
    LogicalStreamType(LengthType type) noexcept
        : lengthType(type) {}

    LogicalStreamType() = delete;
    LogicalStreamType(const LogicalStreamType&) = delete;
    LogicalStreamType(LogicalStreamType&&) noexcept = default;

    const std::optional<DictionaryType>& getDictionaryType() const noexcept { return dictionaryType; }
    const std::optional<OffsetType>& getOffsetType() const noexcept { return offsetType; }
    const std::optional<LengthType>& getLengthType() const noexcept { return lengthType; }

private:
    std::optional<DictionaryType> dictionaryType;
    std::optional<OffsetType> offsetType;
    std::optional<LengthType> lengthType;
};

class StreamMetadata {
public:
    StreamMetadata(PhysicalStreamType physicalStreamType_,
                   std::optional<LogicalStreamType> logicalStreamType_,
                   LogicalLevelTechnique logicalLevelTechnique1_,
                   LogicalLevelTechnique logicalLevelTechnique2_,
                   PhysicalLevelTechnique physicalLevelTechnique_,
                   std::uint32_t numValues_,
                   std::uint32_t byteLength_) noexcept
        : physicalStreamType(physicalStreamType_),
          logicalStreamType(std::move(logicalStreamType_)),
          logicalLevelTechnique1(logicalLevelTechnique1_),
          logicalLevelTechnique2(logicalLevelTechnique2_),
          physicalLevelTechnique(physicalLevelTechnique_),
          numValues(numValues_),
          byteLength(byteLength_) {}
    virtual ~StreamMetadata() = default;

    virtual LogicalLevelTechnique getMetadataType() const noexcept { return LogicalLevelTechnique::NONE; }

    static std::unique_ptr<StreamMetadata> decode(BufferStream&) noexcept(false);

    StreamMetadata() = delete;
    StreamMetadata(const StreamMetadata&) = delete;
    StreamMetadata(StreamMetadata&&) noexcept = default;

    PhysicalStreamType getPhysicalStreamType() const { return physicalStreamType; }
    const std::optional<LogicalStreamType>& getLogicalStreamType() const { return logicalStreamType; }
    LogicalLevelTechnique getLogicalLevelTechnique1() const { return logicalLevelTechnique1; }
    LogicalLevelTechnique getLogicalLevelTechnique2() const { return logicalLevelTechnique2; }
    PhysicalLevelTechnique getPhysicalLevelTechnique() const { return physicalLevelTechnique; }

    std::uint32_t getNumValues() const noexcept { return numValues; }
    std::uint32_t getByteLength() const noexcept { return byteLength; }

private:
    int getLogicalType() const noexcept;

    friend class RleEncodedStreamMetadata;
    friend class MortonEncodedStreamMetadata;
    friend std::unique_ptr<StreamMetadata> decode(BufferStream&);
    static StreamMetadata decodeInternal(BufferStream&);

    PhysicalStreamType physicalStreamType;
    std::optional<LogicalStreamType> logicalStreamType;
    LogicalLevelTechnique logicalLevelTechnique1;
    LogicalLevelTechnique logicalLevelTechnique2;
    PhysicalLevelTechnique physicalLevelTechnique;

    // After logical Level technique was applied -> when rle is used it is the length of the runs and values array
    std::uint32_t numValues;
    std::uint32_t byteLength;
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
    RleEncodedStreamMetadata(PhysicalStreamType physicalStreamType_,
                             std::optional<LogicalStreamType> logicalStreamType_,
                             LogicalLevelTechnique logicalLevelTechnique1_,
                             LogicalLevelTechnique logicalLevelTechnique2_,
                             PhysicalLevelTechnique physicalLevelTechnique_,
                             int numValues_,
                             int byteLength_,
                             int runs_,
                             int numRleValues_) noexcept
        : StreamMetadata(physicalStreamType_,
                         std::move(logicalStreamType_),
                         logicalLevelTechnique1_,
                         logicalLevelTechnique2_,
                         physicalLevelTechnique_,
                         numValues_,
                         byteLength_),
          runs(runs_),
          numRleValues(numRleValues_) {}

    RleEncodedStreamMetadata(StreamMetadata&& streamMetadata, int runs_, int numRleValues_) noexcept
        : StreamMetadata(std::move(streamMetadata)),
          runs(runs_),
          numRleValues(numRleValues_) {}

    RleEncodedStreamMetadata() = delete;
    RleEncodedStreamMetadata(const RleEncodedStreamMetadata&) = delete;
    RleEncodedStreamMetadata(RleEncodedStreamMetadata&&) noexcept = default;

    LogicalLevelTechnique getMetadataType() const noexcept override { return LogicalLevelTechnique::RLE; }

    static RleEncodedStreamMetadata decodePartial(StreamMetadata&& streamMetadata,
                                                  BufferStream& buffer) noexcept(false) {
        const auto [runs, numValues] = util::decoding::decodeVarints<std::uint32_t, 2>(buffer);
        return RleEncodedStreamMetadata(std::move(streamMetadata), runs, numValues);
    }

    static RleEncodedStreamMetadata decode(BufferStream& buffer) noexcept(false) {
        return decodePartial(decodeInternal(buffer), buffer);
    }

    int getRuns() const noexcept { return runs; }
    int getNumRleValues() const noexcept { return numRleValues; }

private:
    int runs;
    int numRleValues;
};

class MortonEncodedStreamMetadata : public StreamMetadata {
public:
    MortonEncodedStreamMetadata(PhysicalStreamType physicalStreamType_,
                                LogicalStreamType logicalStreamType_,
                                LogicalLevelTechnique logicalLevelTechnique1_,
                                LogicalLevelTechnique logicalLevelTechnique2_,
                                PhysicalLevelTechnique physicalLevelTechnique_,
                                int numValues_,
                                int byteLength_,
                                int numBits_,
                                int coordinateShift_) noexcept
        : StreamMetadata(physicalStreamType_,
                         std::move(logicalStreamType_),
                         logicalLevelTechnique1_,
                         logicalLevelTechnique2_,
                         physicalLevelTechnique_,
                         numValues_,
                         byteLength_),
          numBits(numBits_),
          coordinateShift(coordinateShift_) {}

    MortonEncodedStreamMetadata(StreamMetadata&& streamMetadata, int numBits_, int coordinateShift_) noexcept
        : StreamMetadata(std::move(streamMetadata)),
          numBits(numBits_),
          coordinateShift(coordinateShift_) {}

    LogicalLevelTechnique getMetadataType() const noexcept override { return LogicalLevelTechnique::MORTON; }

    static MortonEncodedStreamMetadata decodePartial(StreamMetadata&& streamMetadata,
                                                     BufferStream& buffer) noexcept(false) {
        const auto [numBits, coordShift] = util::decoding::decodeVarints<std::uint32_t, 2>(buffer);
        return MortonEncodedStreamMetadata(std::move(streamMetadata), numBits, coordShift);
    }

    static MortonEncodedStreamMetadata decode(BufferStream& buffer) noexcept(false) {
        return decodePartial(decodeInternal(buffer), buffer);
    }

    int getNumBits() const noexcept { return numBits; }
    int getCoordinateShift() const noexcept { return coordinateShift; }

private:
    int numBits;
    int coordinateShift;
};

class PdeEncodedMetadata {};

} // namespace mlt::metadata::stream
