#include <mlt/metadata/tileset.hpp>
#include <mlt/metadata/tileset_decode.hpp>
#include <mlt/metadata/type_map.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/encoding/varint.hpp>
#include <mlt/util/memory_budget.hpp>
#include <mlt/util/stl.hpp>
#include <mlt/util/varint.hpp>

#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace mlt::metadata::tileset {
using util::decoding::decodeVarint;

namespace {
// Columns can nest, but not deeply. This limit protects the stack, as the nesting comes from the tile.
constexpr unsigned maxColumnDepth = 16;

std::string decodeString(BufferStream& tileData) {
    std::string result;
    const auto length = decodeVarint<std::uint32_t>(tileData);
    if (length > 0) {
        // Check the length before allocating for it
        if (!tileData.available(length)) {
            throw std::runtime_error("Unexpected end of buffer");
        }
        result.resize(length);
        tileData.read(result.data(), length);
    }
    return result;
}

// Decode a number of columns, and check and charge for them before they are allocated.
// Each column takes at least a byte of the tile, but `sizeof(Column)` bytes of memory, so checking the count against
// the remaining data is not enough.
std::uint32_t decodeColumnCount(BufferStream& tileData, util::MemoryBudget& budget) {
    const auto count = decodeVarint<std::uint32_t>(tileData);
    if (count > tileData.getRemaining()) {
        throw std::runtime_error("Unexpected end of buffer");
    }
    budget.consumeItems<Column>(count);
    return count;
}

Column decodeColumn(BufferStream& tileData, util::MemoryBudget& budget, const unsigned depth = 0) {
    if (depth > maxColumnDepth) {
        throw std::runtime_error("Columns are nested too deeply");
    }
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
        if (!column->hasComplexType()) {
            throw std::runtime_error(
                "Column type code indicates children but decoded column does not have complex type");
        }
        auto& complex = column->getComplexType();
        const auto childCount = decodeColumnCount(tileData, budget);
        complex.children = util::generateVector<Column>(childCount, [&](auto) {
            return decodeColumn(tileData, budget, depth + 1);
        });
    }

    return std::move(*column);
}
} // namespace

FeatureTable decodeFeatureTable(BufferStream& tileData, util::MemoryBudget& budget) {
    auto name = decodeString(tileData);
    if (name.empty()) {
        throw std::runtime_error("Missing layer name");
    }
    const auto extent = decodeVarint<std::uint32_t>(tileData);
    const auto columnCount = decodeColumnCount(tileData, budget);
    auto columns = util::generateVector<Column>(columnCount, [&](auto) {
        return decodeColumn(tileData, budget);
    });
    return {.name = std::move(name), .extent = extent, .columns = std::move(columns)};
}

FeatureTable decodeFeatureTable(BufferStream& tileData) {
    util::MemoryBudget budget;
    return decodeFeatureTable(tileData, budget);
}

namespace {
void encodeString(const std::string& str, std::vector<std::uint8_t>& out) {
    util::encoding::encodeVarint(static_cast<std::uint32_t>(str.size()), out);
    out.insert(out.end(), str.begin(), str.end());
}

void encodeColumn(const Column& column, std::vector<std::uint8_t>& out) {
    const bool hasChildren = column.hasComplexType() && column.getComplexType().hasChildren();
    const auto typeCode = std::visit(util::overloaded{
                                         [&](const ScalarColumn& scalar) -> std::optional<std::uint32_t> {
                                             if (scalar.hasPhysicalType()) {
                                                 return type_map::Tag0x01::encodeColumnType(scalar.getPhysicalType(),
                                                                                            std::nullopt,
                                                                                            std::nullopt,
                                                                                            std::nullopt,
                                                                                            column.nullable,
                                                                                            hasChildren,
                                                                                            scalar.hasLongID);
                                             }
                                             return type_map::Tag0x01::encodeColumnType(std::nullopt,
                                                                                        scalar.getLogicalType(),
                                                                                        std::nullopt,
                                                                                        std::nullopt,
                                                                                        column.nullable,
                                                                                        hasChildren,
                                                                                        scalar.hasLongID);
                                         },
                                         [&](const ComplexColumn& complex) -> std::optional<std::uint32_t> {
                                             if (complex.hasPhysicalType()) {
                                                 return type_map::Tag0x01::encodeColumnType(std::nullopt,
                                                                                            std::nullopt,
                                                                                            complex.getPhysicalType(),
                                                                                            std::nullopt,
                                                                                            column.nullable,
                                                                                            hasChildren,
                                                                                            false);
                                             }
                                             return type_map::Tag0x01::encodeColumnType(std::nullopt,
                                                                                        std::nullopt,
                                                                                        std::nullopt,
                                                                                        complex.getLogicalType(),
                                                                                        column.nullable,
                                                                                        hasChildren,
                                                                                        false);
                                         },
                                     },
                                     column.type);

    if (!typeCode) {
        throw std::runtime_error("Cannot encode column type for: " + column.name);
    }
    util::encoding::encodeVarint(*typeCode, out);

    if (type_map::Tag0x01::columnTypeHasName(*typeCode)) {
        encodeString(column.name, out);
    }

    if (type_map::Tag0x01::columnTypeHasChildren(*typeCode)) {
        const auto& children = column.getComplexType().children;
        util::encoding::encodeVarint(static_cast<std::uint32_t>(children.size()), out);
        for (const auto& child : children) {
            encodeColumn(child, out);
        }
    }
}
} // namespace

std::vector<std::uint8_t> encodeFeatureTable(const FeatureTable& table) {
    std::vector<std::uint8_t> result;
    result.reserve(256);

    encodeString(table.name, result);
    util::encoding::encodeVarint(table.extent, result);
    util::encoding::encodeVarint(static_cast<std::uint32_t>(table.columns.size()), result);
    for (const auto& column : table.columns) {
        encodeColumn(column, result);
    }
    return result;
}

} // namespace mlt::metadata::tileset
