#include <mlt/metadata/tileset.hpp>
#include <mlt/metadata/type_map.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/stl.hpp>
#include <mlt/util/varint.hpp>

#include <utility>

namespace mlt::metadata::tileset {
using util::decoding::decodeVarint;

namespace {
std::string decodeString(BufferStream& stream) {
    std::string result;
    const auto length = decodeVarint<std::uint32_t>(stream);
    if (length > 0) {
        result.resize(length);
        stream.read(result.data(), length);
    }
    return result;
}

Column decodeColumn(BufferStream& stream) {
    const auto typeCode = decodeVarint<std::uint32_t>(stream);
    auto column = type_map::Tag0x01::decodeColumnType(typeCode);
    if (!column) {
        // We can't just skip this because we don't know the actual length
        throw std::runtime_error("Unsupported column type code: " + std::to_string(typeCode));
    }

    if (type_map::Tag0x01::columnTypeHasName(typeCode)) {
        column->name = decodeString(stream);
    }

    if (type_map::Tag0x01::columnTypeHasChildren(typeCode)) {
        assert(column->hasComplexType());
        auto& complex = column->getComplexType();
        const auto childCount = decodeVarint<std::uint32_t>(stream);
        complex.children = util::generateVector<Column>(childCount, [&](auto) { return decodeColumn(stream); });
    }

    return *column;
}
} // namespace

FeatureTable decodeFeatureTable(BufferStream& stream) {
    auto name = decodeString(stream);
    const auto columnCount = decodeVarint<std::uint32_t>(stream);
    auto columns = util::generateVector<Column>(columnCount, [&](auto) { return decodeColumn(stream); });
    return {.name = std::move(name), .columns = std::move(columns)};
}

} // namespace mlt::metadata::tileset
