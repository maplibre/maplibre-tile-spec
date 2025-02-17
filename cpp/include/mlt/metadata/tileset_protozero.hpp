#pragma once

#include <mlt/metadata/tileset.hpp>

#include <protozero/pbf_message.hpp>

namespace mlt::metadata::tileset {

namespace detail {
inline bool read(ScalarField&, protozero::pbf_message<schema::ScalarField>);
inline bool read(ComplexField&, protozero::pbf_message<schema::ComplexField>);
inline bool read(ScalarColumn&, protozero::pbf_message<schema::ScalarColumn>);
inline bool read(Field&, protozero::pbf_message<schema::Field>);
inline bool read(ComplexColumn&, protozero::pbf_message<schema::ComplexColumn>);
inline bool read(Column&, protozero::pbf_message<schema::Column>);
inline bool read(FeatureTableSchema&, protozero::pbf_message<schema::FeatureTableSchema>);
} // namespace detail

/// Decode the tileset metadata from a protobuf message
/// @throws protobuf::exception corrupted/truncated input data
std::optional<TileSetMetadata> read(protozero::pbf_message<schema::TileSetMetadata>);

namespace detail {
bool read(ScalarField& field, protozero::pbf_message<schema::ScalarField> message) {
    bool hasPhysical = false;
    bool hasLogical = false;
    while (message.next()) {
        switch (message.tag()) {
            case schema::ScalarField::oneof_ScalarType_physicalType:
                field.type.emplace<ScalarType>(static_cast<ScalarType>(message.get_enum()));
                hasPhysical = true;
                break;
            case schema::ScalarField::oneof_LogicalScalarType_logicalType:
                field.type.emplace<LogicalScalarType>(static_cast<LogicalScalarType>(message.get_enum()));
                hasLogical = true;
                break;
            default:
                message.skip();
                break;
        }
    }
    return (hasLogical != hasPhysical);
}

bool read(ComplexField& field, protozero::pbf_message<schema::ComplexField> message) {
    bool hasPhysical = false;
    bool hasLogical = false;

    while (message.next()) {
        switch (message.tag()) {
            case schema::ComplexField::oneof_ComplexType_physicalType:
                field.type.emplace<ComplexType>(static_cast<ComplexType>(message.get_enum()));
                hasPhysical = true;
                break;
            case schema::ComplexField::oneof_LogicalComplexType_logicalType:
                field.type.emplace<LogicalComplexType>(static_cast<LogicalComplexType>(message.get_enum()));
                hasLogical = true;
                break;
            case schema::ComplexField::repeated_Field_children:
                message.length();
                field.children.resize(field.children.size() + 1);
                if (!read(field.children.back(), message.get_message())) {
                    return false;
                }
                break;
            default:
                message.skip();
                break;
        }
    }
    return (hasLogical != hasPhysical);
}

bool read(ScalarColumn& column, protozero::pbf_message<schema::ScalarColumn> message) {
    bool hasPhysical = false;
    bool hasLogical = false;

    while (message.next()) {
        switch (message.tag()) {
            case schema::ScalarColumn::oneof_ScalarType_physicalType:
                column.type.emplace<ScalarType>(static_cast<ScalarType>(message.get_enum()));
                hasPhysical = true;
                break;
            case schema::ScalarColumn::oneof_LogicalScalarType_logicalType:
                column.type.emplace<LogicalScalarType>(static_cast<LogicalScalarType>(message.get_enum()));
                hasLogical = true;
                break;
            default:
                message.skip();
                break;
        }
    }
    return (hasLogical != hasPhysical);
}

bool read(Field& field, protozero::pbf_message<schema::Field> message) {
    bool hasScalar = false;
    bool hasComplex = false;

    while (message.next()) {
        switch (message.tag()) {
            case schema::Field::optional_string_name:
                field.name = message.get_string();
                break;
            case schema::Field::optional_bool_nullable:
                field.nullable = message.get_bool();
                break;
            case schema::Field::oneof_ScalarField_scalarField:
                if (!read(field.field.emplace<ScalarField>(), message.get_message())) {
                    return false;
                }
                hasScalar = true;
                break;
            case schema::Field::oneof_ComplexField_complexField:
                if (!read(field.field.emplace<ComplexField>(), message.get_message())) {
                    return false;
                }
                hasComplex = true;
                break;
            default:
                message.skip();
                break;
        }
    }
    return (hasScalar != hasComplex);
}

bool read(ComplexColumn& column, protozero::pbf_message<schema::ComplexColumn> message) {
    bool hasPhysical = false;
    bool hasLogical = false;

    while (message.next()) {
        switch (message.tag()) {
            case schema::ComplexColumn::oneof_ComplexType_physicalType:
                column.type.emplace<ComplexType>(static_cast<ComplexType>(message.get_enum()));
                hasPhysical = true;
                break;
            case schema::ComplexColumn::oneof_LogicalComplexType_logicalType:
                column.type.emplace<LogicalComplexType>(static_cast<LogicalComplexType>(message.get_enum()));
                hasLogical = true;
                break;
            case schema::ComplexColumn::repeated_Field_children:
                column.children.resize(column.children.size() + 1);
                if (!read(column.children.back(), message.get_message())) {
                    return false;
                }
                break;
            default:
                message.skip();
                break;
        }
    }
    return (hasLogical != hasPhysical);
}

bool read(Column& column, protozero::pbf_message<schema::Column> message) {
    bool hasScalar = false;
    bool hasComplex = false;

    while (message.next()) {
        switch (message.tag()) {
            case schema::Column::implicit_string_name: // treated as required
                column.name = message.get_string();
                break;
            case schema::Column::implicit_bool_nullable:
                column.nullable = message.get_bool();
                break;
            case schema::Column::implicit_ColumnScope_columnScope:
                column.columnScope = static_cast<ColumnScope>(message.get_enum());
                break;
            case schema::Column::oneof_ScalarColumn_scalarType:
                if (!read(column.type.emplace<ScalarColumn>(), message.get_message())) {
                    return false;
                }
                hasScalar = true;
                break;
            case schema::Column::oneof_ComplexColumn_complexType:
                if (!read(column.type.emplace<ComplexColumn>(), message.get_message())) {
                    return false;
                }
                hasComplex = true;
                break;
            default:
                message.skip();
                break;
        }
    }

    return !column.name.empty() && (hasScalar != hasComplex);
}

bool read(FeatureTableSchema& schema, protozero::pbf_message<schema::FeatureTableSchema> message) {
    while (message.next()) {
        switch (message.tag()) {
            case schema::FeatureTableSchema::implicit_string_name: // treated as
                                                                   // required
                schema.name = message.get_string();
                break;
            case schema::FeatureTableSchema::repeated_Column_columns:
                schema.columns.resize(schema.columns.size() + 1);
                if (!read(schema.columns.back(), message.get_message())) {
                    return false;
                }
                break;
            default:
                message.skip();
                break;
        }
    }
    return !schema.name.empty() && !schema.columns.empty();
}
} // namespace detail

inline std::optional<TileSetMetadata> read(protozero::pbf_message<schema::TileSetMetadata> message) {
    int boundCount = 0;
    int centerCount = 0;
    TileSetMetadata result;

    while (message.next()) {
        switch (message.tag()) {
            case schema::TileSetMetadata::implicit_int32_version: {
                result.version = message.get_int32();

                constexpr auto minVersion = 1;
                if (result.version < minVersion) {
                    return {};
                }
                break;
            }
            case schema::TileSetMetadata::repeated_FeatureTableSchema_featureTables:
                result.featureTables.resize(result.featureTables.size() + 1);
                if (!detail::read(result.featureTables.back(), message.get_message())) {
                    return {};
                }
                break;
            case schema::TileSetMetadata::optional_string_name:
                result.name = message.get_string();
                break;
            case schema::TileSetMetadata::optional_string_description:
                result.description = message.get_string();
                break;
            case schema::TileSetMetadata::optional_string_attribution:
                result.attribution = message.get_string();
                break;
            case schema::TileSetMetadata::optional_int32_minZoom:
                result.minZoom = message.get_int32();
                break;
            case schema::TileSetMetadata::optional_int32_maxZoom:
                result.maxZoom = message.get_int32();
                break;
            case schema::TileSetMetadata::repeated_double_bounds:
                switch (boundCount++) {
                    case 0:
                        result.boundLeft = message.get_double();
                        break;
                    case 1:
                        result.boundBottom = message.get_double();
                        break;
                    case 2:
                        result.boundRight = message.get_double();
                        break;
                    case 3:
                        result.boundTop = message.get_double();
                        break;
                    default:
                        return {};
                }
                break;
            case schema::TileSetMetadata::repeated_double_center:
                switch (centerCount++) {
                    case 0:
                        result.centerLon = message.get_double();
                        break;
                    case 1:
                        result.centerLat = message.get_double();
                        break;
                    default:
                        return {};
                }
                break;
            default:
                message.skip();
                break;
        }
    }
    return result;
}

} // namespace mlt::metadata::tileset
