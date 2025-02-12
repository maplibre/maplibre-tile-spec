#include <mlt/metadata/tileset.hpp>

#include <optional>
#include <variant>
#include <vector>

namespace mlt::metadata::tileset {

bool ScalarField::read(protozero::pbf_message<schema::ScalarField> message) {
    bool hasPhysical = false;
    bool hasLogical = false;
    while (message.next()) {
        switch (message.tag()) {
            case schema::ScalarField::oneof_ScalarType_physicalType:
                type.emplace<ScalarType>(static_cast<ScalarType>(message.get_enum()));
                hasPhysical = true;
                break;
            case schema::ScalarField::oneof_LogicalScalarType_logicalType:
                type.emplace<LogicalScalarType>(static_cast<LogicalScalarType>(message.get_enum()));
                hasLogical = true;
                break;
            default:
                message.skip();
                break;
        }
    }
    return (hasLogical != hasPhysical);
}

bool ComplexField::read(protozero::pbf_message<schema::ComplexField> message) {
    bool hasPhysical = false;
    bool hasLogical = false;

    while (message.next()) {
        switch (message.tag()) {
            case schema::ComplexField::oneof_ComplexType_physicalType:
                type.emplace<ComplexType>(static_cast<ComplexType>(message.get_enum()));
                hasPhysical = true;
                break;
            case schema::ComplexField::oneof_LogicalComplexType_logicalType:
                type.emplace<LogicalComplexType>(static_cast<LogicalComplexType>(message.get_enum()));
                hasLogical = true;
                break;
            case schema::ComplexField::repeated_Field_children:
                message.length();
                children.resize(children.size() + 1);
                if (!children.back().read(message.get_message())) {
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

bool ScalarColumn::read(protozero::pbf_message<schema::ScalarColumn> message) {
    bool hasPhysical = false;
    bool hasLogical = false;

    while (message.next()) {
        switch (message.tag()) {
            case schema::ScalarColumn::oneof_ScalarType_physicalType:
                type.emplace<ScalarType>(static_cast<ScalarType>(message.get_enum()));
                hasPhysical = true;
                break;
            case schema::ScalarColumn::oneof_LogicalScalarType_logicalType:
                type.emplace<LogicalScalarType>(static_cast<LogicalScalarType>(message.get_enum()));
                hasLogical = true;
                break;
            default:
                message.skip();
                break;
        }
    }
    return (hasLogical != hasPhysical);
}

bool Field::read(protozero::pbf_message<schema::Field> message) {
    bool hasScalar = false;
    bool hasComplex = false;

    while (message.next()) {
        switch (message.tag()) {
            case schema::Field::optional_string_name:
                name = message.get_string();
                break;
            case schema::Field::optional_bool_nullable:
                nullable = message.get_bool();
                break;
            case schema::Field::oneof_ScalarField_scalarField:
                if (!field.emplace<ScalarField>().read(message.get_message())) {
                    return false;
                }
                hasScalar = true;
                break;
            case schema::Field::oneof_ComplexField_complexField:
                if (!field.emplace<ComplexField>().read(message.get_message())) {
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

bool ComplexColumn::read(protozero::pbf_message<schema::ComplexColumn> message) {
    bool hasPhysical = false;
    bool hasLogical = false;

    while (message.next()) {
        switch (message.tag()) {
            case schema::ComplexColumn::oneof_ComplexType_physicalType:
                type.emplace<ComplexType>(static_cast<ComplexType>(message.get_enum()));
                hasPhysical = true;
                break;
            case schema::ComplexColumn::oneof_LogicalComplexType_logicalType:
                type.emplace<LogicalComplexType>(static_cast<LogicalComplexType>(message.get_enum()));
                hasLogical = true;
                break;
            case schema::ComplexColumn::repeated_Field_children:
                children.resize(children.size() + 1);
                if (!children.back().read(message.get_message())) {
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

bool Column::read(protozero::pbf_message<schema::Column> message) {
    bool hasScalar = false;
    bool hasComplex = false;

    while (message.next()) {
        switch (message.tag()) {
            case schema::Column::implicit_string_name: // treated as required
                name = message.get_string();
                break;
            case schema::Column::implicit_bool_nullable:
                nullable = message.get_bool();
                break;
            case schema::Column::implicit_ColumnScope_columnScope:
                columnScope = static_cast<ColumnScope>(message.get_enum());
                break;
            case schema::Column::oneof_ScalarColumn_scalarType:
                if (!type.emplace<ScalarColumn>().read(message.get_message())) {
                    return false;
                }
                hasScalar = true;
                break;
            case schema::Column::oneof_ComplexColumn_complexType:
                if (!type.emplace<ComplexColumn>().read(message.get_message())) {
                    return false;
                }
                hasComplex = true;
                break;
            default:
                message.skip();
                break;
        }
    }

    return !name.empty() && (hasScalar != hasComplex);
}

bool FeatureTableSchema::read(protozero::pbf_message<schema::FeatureTableSchema> message) {
    while (message.next()) {
        switch (message.tag()) {
            case schema::FeatureTableSchema::implicit_string_name: // treated as
                                                                   // required
                name = message.get_string();
                break;
            case schema::FeatureTableSchema::repeated_Column_columns:
                columns.resize(columns.size() + 1);
                if (!columns.back().read(message.get_message())) {
                    return false;
                }
                break;
            default:
                message.skip();
                break;
        }
    }
    return !name.empty() && !columns.empty();
}

bool TileSetMetadata::read(protozero::pbf_message<schema::TileSetMetadata> message) {
    int boundCount = 0;
    int centerCount = 0;

    while (message.next()) {
        switch (message.tag()) {
            case schema::TileSetMetadata::implicit_int32_version: {
                version = message.get_int32();

                constexpr auto minVersion = 1;
                if (version < minVersion) {
                    return false;
                }
                break;
            }
            case schema::TileSetMetadata::repeated_FeatureTableSchema_featureTables:
                featureTables.resize(featureTables.size() + 1);
                if (!featureTables.back().read(message.get_message())) {
                    return false;
                }
                break;
            case schema::TileSetMetadata::optional_string_name:
                name = message.get_string();
                break;
            case schema::TileSetMetadata::optional_string_description:
                description = message.get_string();
                break;
            case schema::TileSetMetadata::optional_string_attribution:
                attribution = message.get_string();
                break;
            case schema::TileSetMetadata::optional_int32_minZoom:
                minZoom = message.get_int32();
                break;
            case schema::TileSetMetadata::optional_int32_maxZoom:
                maxZoom = message.get_int32();
                break;
            case schema::TileSetMetadata::repeated_double_bounds:
                switch (boundCount++) {
                    case 0:
                        boundLeft = message.get_double();
                        break;
                    case 1:
                        boundBottom = message.get_double();
                        break;
                    case 2:
                        boundRight = message.get_double();
                        break;
                    case 3:
                        boundTop = message.get_double();
                        break;
                    default:
                        return false;
                }
                break;
            case schema::TileSetMetadata::repeated_double_center:
                switch (centerCount++) {
                    case 0:
                        centerLon = message.get_double();
                        break;
                    case 1:
                        centerLat = message.get_double();
                        break;
                    default:
                        return false;
                }
                break;
            default:
                message.skip();
                break;
        }
    }
    return !featureTables.empty();
}

} // namespace mlt::metadata::tileset
