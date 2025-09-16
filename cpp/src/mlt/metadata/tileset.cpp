#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/varint.hpp>
#include <utility>

namespace mlt::metadata::tileset {
using namespace util::decoding;

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

Field decodeField(BufferStream& stream) {
    Field field;
    field.name = decodeString(stream);

    const auto fieldOptions = decodeVarint<std::uint32_t>(stream);
    const auto logical = !!(fieldOptions & std::to_underlying(schema::FieldOptions::logicalType));
    field.nullable = (fieldOptions & std::to_underlying(schema::FieldOptions::nullable)) != 0;

    const auto typeValue = decodeVarint<ScalarType>(stream);

    if (fieldOptions & std::to_underlying(schema::FieldOptions::complexType)) {
        ComplexField child;
        if (logical) {
            child.type = static_cast<schema::LogicalComplexType>(typeValue);
        } else {
            child.type = static_cast<schema::ComplexType>(typeValue);
        }
        if (fieldOptions & std::to_underlying(schema::FieldOptions::hasChildren)) {
            const auto childCount = decodeVarint<std::uint32_t>(stream);
            child.children.reserve(childCount);
            for (std::uint32_t k = 0; k < childCount; ++k) {
                child.children.push_back(decodeField(stream));
            }
        }
        field.field = std::move(child);
    } else {
        if (logical) {
            field.field = ScalarField{.type = static_cast<schema::LogicalScalarType>(typeValue)};
        } else {
            field.field = ScalarField{.type = static_cast<schema::ScalarType>(typeValue)};
        }
    }

    return field;
}

std::variant<ScalarColumn, ComplexColumn> decodeColumn(BufferStream& stream, std::uint8_t columnOptions) {
    const auto logical = !!(columnOptions & std::to_underlying(schema::ColumnOptions::logicalType));
    const auto typeValue = decodeVarint<ScalarType>(stream);
    if (columnOptions & std::to_underlying(schema::ColumnOptions::complexType)) {
        ComplexColumn complexColumn;
        if (logical) {
            complexColumn.type = static_cast<schema::LogicalComplexType>(typeValue);
        } else {
            complexColumn.type = static_cast<schema::ComplexType>(typeValue);
        }
        if (columnOptions & std::to_underlying(schema::ColumnOptions::hasChildren)) {
            const auto childCount = decodeVarint<std::uint32_t>(stream);
            complexColumn.children.reserve(childCount);
            for (std::uint32_t k = 0; k < childCount; ++k) {
                complexColumn.children.push_back(decodeField(stream));
            }
        }
        return complexColumn;
    } else {
        if (logical) {
            return ScalarColumn{.type = static_cast<schema::LogicalScalarType>(typeValue)};
        } else {
            return ScalarColumn{.type = static_cast<schema::ScalarType>(typeValue)};
        }
    }
}

} // namespace

TileMetadata decodeTileMetadata(const BufferStream& stream) {
    BufferStream copy{stream.getRemainingView()};
    return decodeTileMetadata(copy);
}

TileMetadata decodeTileMetadata(BufferStream& stream) {
    std::vector<FeatureTable> featureTables;
    while (stream.getRemaining()) {
        FeatureTable featureTable;
        featureTable.name = decodeString(stream);

        const auto columnCount = decodeVarint<std::uint32_t>(stream);
        featureTable.columns.reserve(columnCount);
        for (std::uint32_t j = 0; j < columnCount; ++j) {
            const auto columnOptions = decodeVarint<std::uint32_t>(stream);

            Column column;
            column.name = decodeString(stream);
            column.nullable = (columnOptions & std::to_underlying(schema::ColumnOptions::nullable)) != 0;
            column.columnScope = (columnOptions & std::to_underlying(schema::ColumnOptions::vertexScope))
                                     ? ColumnScope::VERTEX
                                     : ColumnScope::FEATURE;
            column.type = decodeColumn(stream, columnOptions);

            featureTable.columns.push_back(std::move(column));
        }
        featureTables.push_back(std::move(featureTable));
    }
    return {.featureTables = std::move(featureTables)};
}

} // namespace mlt::metadata::tileset
