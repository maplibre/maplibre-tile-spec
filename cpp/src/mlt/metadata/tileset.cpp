#include <mlt/metadata/tileset.hpp>
#include <mlt/metadata/type_map.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/stl.hpp>
#include <mlt/util/varint.hpp>

#include <utility>

namespace mlt::metadata::tileset {
using util::decoding::decodeVarint;

namespace {
std::string decodeString(BufferStream& tileData) {
    std::string result;
    const auto length = decodeVarint<std::uint32_t>(tileData);
    if (length > 0) {
        result.resize(length);
        tileData.read(result.data(), length);
    }
    return result;
}

Column decodeColumn(BufferStream& tileData) {
    const auto typeCode = decodeVarint<std::uint32_t>(tileData);
    auto column = type_map::Tag0x01::decodeColumnType(typeCode);
    if (!column) {
        // We can't just skip this because we don't know the actual length
        throw std::runtime_error("Unsupported column type code: " + std::to_string(typeCode));
    }

    if (type_map::Tag0x01::columnTypeHasName(typeCode)) {
        column->name = decodeString(tileData);
    }

    if (type_map::Tag0x01::columnTypeHasChildren(typeCode)) {
        assert(column->hasComplexType());
        auto& complex = column->getComplexType();
        const auto childCount = decodeVarint<std::uint32_t>(tileData);
        complex.children = util::generateVector<Column>(childCount, [&](auto) { return decodeColumn(tileData); });
    }

    return *column;
}
} // namespace

FeatureTable decodeFeatureTable(BufferStream& tileData) {
    auto name = decodeString(tileData);
    const auto extent = decodeVarint<std::uint32_t>(tileData);
    const auto columnCount = decodeVarint<std::uint32_t>(tileData);
    auto columns = util::generateVector<Column>(columnCount, [&](auto) { return decodeColumn(tileData); });
    return {.name = std::move(name), .extent = extent, .columns = std::move(columns)};
}

} // namespace mlt::metadata::tileset
