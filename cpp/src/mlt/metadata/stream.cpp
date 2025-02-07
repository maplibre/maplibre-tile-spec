#include <mlt/metadata/stream.hpp>

#include <utility>

namespace mlt::metadata::stream {

namespace {
std::optional<LogicalStreamType> decodeLogicalStreamType(PhysicalStreamType physicalStreamType, int value) noexcept {
    switch (physicalStreamType) {
        case PhysicalStreamType::DATA:
            return static_cast<DictionaryType>(value);
        case PhysicalStreamType::OFFSET:
            return static_cast<OffsetType>(value);
        case PhysicalStreamType::LENGTH:
            return static_cast<LengthType>(value);
        default:
        case PhysicalStreamType::PRESENT:
            return {};
    }
}
} // namespace

std::unique_ptr<StreamMetadata> StreamMetadata::decode(BufferStream& buffer) {
    auto streamMetadata = decodeInternal(buffer);

    // Currently Morton can't be combined with RLE only with delta
    if (streamMetadata.getLogicalLevelTechnique1() == LogicalLevelTechnique::MORTON) {
        auto result = MortonEncodedStreamMetadata::decodePartial(std::move(streamMetadata), buffer);
        return std::make_unique<MortonEncodedStreamMetadata>(std::move(result));
    }
    // Boolean RLE doesn't need additional information
    else if ((streamMetadata.getLogicalLevelTechnique1() == LogicalLevelTechnique::RLE ||
              streamMetadata.getLogicalLevelTechnique2() == LogicalLevelTechnique::RLE) &&
             streamMetadata.getPhysicalLevelTechnique() != PhysicalLevelTechnique::NONE) {
        auto result = RleEncodedStreamMetadata::decodePartial(std::move(streamMetadata), buffer);
        return std::make_unique<RleEncodedStreamMetadata>(std::move(result));
    }
    return std::make_unique<StreamMetadata>(std::move(streamMetadata));
}

int StreamMetadata::getLogicalType() const noexcept {
    if (logicalStreamType) {
        if (logicalStreamType->getDictionaryType()) {
            return std::to_underlying(*logicalStreamType->getDictionaryType());
        }

        if (logicalStreamType->getLengthType()) {
            return std::to_underlying(*logicalStreamType->getLengthType());
        }

        if (logicalStreamType->getOffsetType()) {
            return std::to_underlying(*logicalStreamType->getOffsetType());
        }
    }
    return 0;
}

StreamMetadata StreamMetadata::decodeInternal(BufferStream& buffer) {
    const auto streamType = buffer.read();
    const auto physicalStreamType = static_cast<PhysicalStreamType>(streamType >> 4);
    auto logicalStreamType = decodeLogicalStreamType(physicalStreamType, streamType & 0x0f);

    const auto encodingsHeader = buffer.read() & 0xff;
    const auto logicalLevelTechnique1 = static_cast<LogicalLevelTechnique>(encodingsHeader >> 5);
    const auto logicalLevelTechnique2 = static_cast<LogicalLevelTechnique>((encodingsHeader >> 2) & 0x7);
    const auto physicalLevelTechnique = static_cast<PhysicalLevelTechnique>(encodingsHeader & 0x3);

    using namespace util::decoding;
    const auto [numValues, byteLength] = decodeVarints<std::uint32_t, 2>(buffer);

    return {
        physicalStreamType,
        std::move(logicalStreamType),
        logicalLevelTechnique1,
        logicalLevelTechnique2,
        physicalLevelTechnique,
        numValues,
        byteLength,
    };
}

} // namespace mlt::metadata::stream
