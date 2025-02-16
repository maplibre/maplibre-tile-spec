use std::u64;

use crate::converter::mvt::MapboxVectorTile;
use crate::error::MltError;
use crate::metadata::proto_tileset::{
    column, complex_column::Type::PhysicalType, field, scalar_column, Column, ColumnScope,
    ComplexColumn, ComplexType, FeatureTableSchema, Field, ScalarColumn, ScalarType,
    TileSetMetadata,
};
use crate::metadata::proto_tileset::{scalar_field, ScalarField};
use crate::MltResult;
use geozero::mvt;
use indexmap::IndexMap;

use super::mvt::ColumnMapping;

const VERSION: i32 = 1;
const ID_COLUMN_NAME: &str = "id";
const GEOMETRY_COLUMN_NAME: &str = "geometry";

fn create_tileset_metadata(
    mvt: MapboxVectorTile,
    is_id_present: bool,
    column_mappings: Option<Vec<ColumnMapping>>,
) -> MltResult<TileSetMetadata> {
    let mut tileset = TileSetMetadata {
        version: VERSION,
        name: None,
        description: None,
        feature_tables: Vec::new(),
        attribution: None,
        min_zoom: None,
        max_zoom: None,
        bounds: Vec::new(),
        center: Vec::new(),
    };

    for layer in &mvt.layers {
        let mut feature_table_scheme: IndexMap<String, Column> = IndexMap::new();

        if is_id_present {
            let id_metadata = Column {
                name: ID_COLUMN_NAME.to_string(),
                nullable: false,
                column_scope: ColumnScope::Feature.into(),
                r#type: {
                    if layer.features.iter().all(|f| match f.id {
                        Some(id) => id <= i32::MAX as u64,
                        None => false,
                    }) {
                        Some(column::Type::ScalarType(ScalarColumn {
                            r#type: Some(scalar_column::Type::PhysicalType(
                                ScalarType::Uint32 as i32,
                            )),
                        }))
                    } else {
                        Some(column::Type::ScalarType(ScalarColumn {
                            r#type: Some(scalar_column::Type::PhysicalType(
                                ScalarType::Uint64 as i32,
                            )),
                        }))
                    }
                },
            };
            feature_table_scheme.insert(ID_COLUMN_NAME.to_string(), id_metadata);
        }

        let geometry_data = Column {
            name: GEOMETRY_COLUMN_NAME.to_string(),
            nullable: false,
            column_scope: ColumnScope::Feature.into(),
            r#type: Some(column::Type::ComplexType(ComplexColumn {
                children: Vec::new(),
                r#type: Some(PhysicalType(ComplexType::Geometry as i32)),
            })),
        };
        feature_table_scheme.insert(GEOMETRY_COLUMN_NAME.to_string(), geometry_data);

        let mut properties: Vec<(String, mvt::tile::Value)> = layer
            .keys
            .iter()
            .zip(layer.values.iter())
            .map(|(k, v)| (k.replace("_", ":"), v.clone()))
            .collect();
        properties.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

        let mut complex_property_column_schemes: IndexMap<String, ComplexColumn> = IndexMap::new();

        for (property_key, property_value) in properties.iter() {
            if feature_table_scheme.contains_key(property_key) {
                continue;
            }
            let scalar_type = get_scalar_type(&property_value)?;

            // Column Mappings: the Java code does not have a unit test for this block of code.
            // Hard to determine the expected behavior.
            if column_mappings.is_some() {
                if column_mappings.as_ref().map_or(false, |mappings| {
                    mappings
                        .iter()
                        .any(|m| property_key == m.mvt_property_prefix.as_str())
                }) && !complex_property_column_schemes.contains_key(property_key)
                {
                    // case where the top-level field is present like name (name:de, name:us, ...) and has a value.
                    // In this case the field is mapped to the name default.
                    let child_field = Field {
                        name: Some("default".to_string()),
                        nullable: Some(true),
                        r#type: Some(field::Type::ScalarField(ScalarField {
                            r#type: Some(scalar_field::Type::PhysicalType(scalar_type as i32)),
                        })),
                    };
                    let field_metadata_builder = ComplexColumn {
                        children: vec![child_field],
                        r#type: Some(PhysicalType(ComplexType::Struct as i32)),
                    };
                    complex_property_column_schemes
                        .insert(property_key.clone(), field_metadata_builder);
                    continue;
                } else if column_mappings.as_ref().map_or(false, |mappings| {
                    mappings
                        .iter()
                        .any(|m| property_key == m.mvt_property_prefix.as_str())
                }) && complex_property_column_schemes.contains_key(property_key)
                    && complex_property_column_schemes
                        .get(property_key)
                        .map_or(false, |column| {
                            column
                                .children
                                .iter()
                                .any(|c| c.name.as_deref() == Some("default"))
                        })
                {
                    // Case where the top-level field such as name is not present in the first feature
                    let child_field = Field {
                        name: Some("default".to_string()),
                        nullable: Some(true),
                        r#type: Some(field::Type::ScalarField(ScalarField {
                            r#type: Some(scalar_field::Type::PhysicalType(scalar_type as i32)),
                        })),
                    };
                    let column_mapping = column_mappings
                        .as_ref()
                        .expect("columnMappings.get() must not be null")
                        .iter()
                        .find(|m| property_key == m.mvt_property_prefix.as_str())
                        .expect("No matching column mapping found");
                    complex_property_column_schemes
                        .get_mut(column_mapping.mvt_property_prefix.as_str())
                        .expect("No matching column mapping found")
                        .children
                        .push(child_field);
                    continue;
                } else if column_mappings.as_ref().map_or(false, |mappings| {
                    mappings.iter().any(|m| {
                        property_key.contains(&m.mvt_property_prefix)
                            && property_key.contains(&m.mvt_delimiter_sign)
                    })
                }) {
                    let column_mapping = column_mappings
                        .as_ref()
                        .expect("columnMappings.get() must not be null")
                        .iter()
                        .find(|m| property_key == m.mvt_property_prefix.as_str())
                        .expect("No matching column mapping found");
                    let field_name = property_key
                        .split(&column_mapping.mvt_delimiter_sign)
                        .nth(1)
                        .expect("No second element found in split");
                    let children = Field {
                        name: Some(field_name.to_string()),
                        nullable: Some(true),
                        r#type: Some(field::Type::ScalarField(ScalarField {
                            r#type: Some(scalar_field::Type::PhysicalType(scalar_type as i32)),
                        })),
                    };
                    if complex_property_column_schemes
                        .contains_key(column_mapping.mvt_property_prefix.as_str())
                    {
                        // add the nested properties to the parent like the name:* properties to the name parent struct
                        if !complex_property_column_schemes
                            .get(column_mapping.mvt_property_prefix.as_str())
                            .expect("No matching column mapping found")
                            .children
                            .iter()
                            .any(|c| c.name.as_deref() == Some(field_name))
                        {
                            complex_property_column_schemes
                                .get_mut(column_mapping.mvt_property_prefix.as_str())
                                .expect("No matching column mapping found")
                                .children
                                .push(children);
                        };
                    } else {
                        // Case where there is no explicit property available which serves as the name
                        // for the top-level field. For example there is no name property only name:*
                        let complex_column_builder = ComplexColumn {
                            children: vec![children],
                            r#type: Some(PhysicalType(ComplexType::Struct as i32)),
                        };
                        complex_property_column_schemes.insert(
                            column_mapping.mvt_property_prefix.clone(),
                            complex_column_builder,
                        );
                    }
                    continue;
                }
            }

            let column_scheme = Column {
                name: property_key.clone(),
                nullable: true,
                column_scope: ColumnScope::Feature.into(),
                r#type: Some(column::Type::ScalarType(ScalarColumn {
                    r#type: Some(scalar_column::Type::PhysicalType(scalar_type as i32)),
                })),
            };
            feature_table_scheme.insert(property_key.clone(), column_scheme);
        }

        let feature_table_schema_builder = FeatureTableSchema {
            name: layer.name.clone(),
            columns: feature_table_scheme.into_iter().map(|(_, v)| v).collect(),
        };

        tileset.feature_tables.push(feature_table_schema_builder);
    }

    Ok(tileset)
}

fn get_scalar_type(value: &mvt::tile::Value) -> MltResult<ScalarType> {
    match value {
        mvt::tile::Value {
            string_value: Some(_),
            ..
        } => Ok(ScalarType::String),
        mvt::tile::Value {
            float_value: Some(_),
            ..
        } => Ok(ScalarType::Float),
        mvt::tile::Value {
            double_value: Some(_),
            ..
        } => Ok(ScalarType::Double),
        mvt::tile::Value {
            int_value: Some(_), ..
        } => Ok(ScalarType::Int32),
        mvt::tile::Value {
            uint_value: Some(_),
            ..
        } => Ok(ScalarType::Uint32),
        mvt::tile::Value {
            bool_value: Some(_),
            ..
        } => Ok(ScalarType::Boolean),
        _ => Err(MltError::UnsupportedKeyType(
            "Unsupported key value type".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::converter::mvt::decode_mvt;

    #[test]
    fn test_create_tileset_metadata() {
        let data = &include_bytes!("../../../../test/fixtures/bing/4-12-6.mvt")[..];
        let tile = decode_mvt(data);
        let metadata = create_tileset_metadata(tile.clone(), true, None);
        assert_eq!(
            metadata.unwrap().feature_tables.len(),
            tile.layers.iter().count()
        );
    }
}
