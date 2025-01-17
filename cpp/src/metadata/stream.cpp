#include <metadata/stream.hpp>

namespace mlt::metadata::stream {

namespace {
std::optional<LogicalStreamType> decodeLogicalStreamType(PhysicalStreamType physicalStreamType, int value) {
    switch (physicalStreamType) {
        case PhysicalStreamType::DATA:
            return static_cast<DictionaryType>(value);
        case PhysicalStreamType::OFFSET:
            return static_cast<OffsetType>(value);
        case PhysicalStreamType::LENGTH:
            return static_cast<LengthType>(value);
        case PhysicalStreamType::PRESENT:
            return {};
    }
}
} // namespace

int StreamMetadata::getLogicalType() {
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

StreamMetadata StreamMetadata::decode(DataView tileData, offset_t& offset) {
    const auto streamType = tileData[offset++];
    const auto physicalStreamType = static_cast<PhysicalStreamType>(streamType >> 4);
    auto logicalStreamType = decodeLogicalStreamType(physicalStreamType, streamType & 0x0f);

    const auto encodingsHeader = tileData[offset++] & 0xff;
    const auto logicalLevelTechnique1 = static_cast<LogicalLevelTechnique>(encodingsHeader >> 5);
    const auto logicalLevelTechnique2 = static_cast<LogicalLevelTechnique>((encodingsHeader >> 2) & 0x7);
    const auto physicalLevelTechnique = static_cast<PhysicalLevelTechnique>(encodingsHeader & 0x3);

    using namespace util::decoding;
    std::int32_t numValues, byteLength;
    decodeVarints(tileData, offset, numValues, byteLength);

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
