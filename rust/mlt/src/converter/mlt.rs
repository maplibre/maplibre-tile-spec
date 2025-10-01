use std::collections::HashMap;

use geo_types::Geometry;
use indexmap::IndexMap;

use crate::converter::mvt::MapVectorTile;
use crate::data::{Feature, Value};
use crate::metadata::proto_tileset::complex_column::Type::PhysicalType;
use crate::metadata::proto_tileset::{
    Column, ColumnScope, ComplexColumn, ComplexType, FeatureTableSchema, Field, ScalarColumn,
    ScalarField, ScalarType, TileSetMetadata, column, field, scalar_column, scalar_field,
};
use crate::metadata::stream_encoding::PhysicalLevelTechnique;
use crate::mvt::ColumnMapping;

struct SortSettings {
    is_sortable: bool,
    feature_ids: Vec<i64>,
}

impl SortSettings {
    fn new(
        is_column_sortable: bool,
        feature_table_optimizations: Option<&FeatureTableOptimizations>,
        ids: Vec<i64>,
    ) -> Self {
        let is_sortable = is_column_sortable
            && feature_table_optimizations.is_some_and(|opt| opt.allow_id_regeneration);

        Self {
            is_sortable,
            feature_ids: ids,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FeatureTableOptimizations {
    pub allow_sorting: bool,
    pub allow_id_regeneration: bool,
    pub column_mappings: Option<Vec<ColumnMapping>>,
}

#[derive(Debug, Clone)]
pub struct ConversionConfig {
    pub include_ids: bool,
    pub use_advanced_encoding_schemes: bool,
    pub optimizations: HashMap<String, FeatureTableOptimizations>,
}

const VERSION: i32 = 1;
const ID_COLUMN_NAME: &str = "id";
const GEOMETRY_COLUMN_NAME: &str = "geometry";

#[must_use]
pub fn create_tileset_metadata(
    mut mvt: MapVectorTile,
    is_id_present: bool,
    column_mappings: Option<&[ColumnMapping]>,
) -> TileSetMetadata {
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

    for layer in &mut mvt.layers {
        let mut feature_table_scheme: IndexMap<String, Column> = IndexMap::new();

        if is_id_present {
            let id_metadata = Column {
                name: ID_COLUMN_NAME.to_string(),
                nullable: false,
                column_scope: ColumnScope::Feature.into(),
                r#type: {
                    if layer.features.iter().all(|f| f.id <= i64::from(i32::MAX)) {
                        Some(column::Type::ScalarType(ScalarColumn {
                            long_id: false,
                            r#type: Some(scalar_column::Type::PhysicalType(
                                ScalarType::Uint32 as i32,
                            )),
                        }))
                    } else {
                        Some(column::Type::ScalarType(ScalarColumn {
                            long_id: true,
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

        let mut complex_property_column_schemes: IndexMap<String, ComplexColumn> = IndexMap::new();

        for feature in &mut layer.features {
            feature.properties.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

            for (property_key, property_value) in &feature.properties {
                if feature_table_scheme.contains_key(property_key) {
                    continue;
                }
                let scalar_type = get_scalar_type(property_value);

                // Column Mappings: the Java code does not have a unit test for this block of code.
                // Hard to determine the expected behavior.
                if let Some(mappings) = column_mappings {
                    if mappings
                        .iter()
                        .any(|m| property_key == m.mvt_property_prefix.as_str())
                        && !complex_property_column_schemes.contains_key(property_key)
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
                    } else if column_mappings.as_ref().is_some_and(|mappings| {
                        mappings
                            .iter()
                            .any(|m| property_key == m.mvt_property_prefix.as_str())
                    }) && complex_property_column_schemes.contains_key(property_key)
                        && complex_property_column_schemes
                            .get(property_key)
                            .is_some_and(|column| {
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
                    } else if column_mappings.as_ref().is_some_and(|mappings| {
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
                            }
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
                        long_id: false, // fixme: not sure if this is correct
                        r#type: Some(scalar_column::Type::PhysicalType(scalar_type as i32)),
                    })),
                };
                feature_table_scheme.insert(property_key.clone(), column_scheme);
            }
        }
        // End of properties loop

        let feature_table_schema_builder = FeatureTableSchema {
            name: layer.name.clone(),
            columns: feature_table_scheme.into_iter().map(|(_, v)| v).collect(),
        };

        tileset.feature_tables.push(feature_table_schema_builder);
    }

    tileset
}

fn get_scalar_type(value: &Value) -> ScalarType {
    match value {
        Value::String(_) => ScalarType::String,
        Value::Float(_) => ScalarType::Float,
        Value::Double(_) => ScalarType::Double,
        Value::Int(_) => ScalarType::Int32,
        Value::Uint(_) => ScalarType::Uint32,
        Value::Bool(_) => ScalarType::Boolean,
    }
}

pub fn convert_mvt(
    mvt: MapVectorTile,
    config: &ConversionConfig,
    tileset_metadata: &TileSetMetadata,
) -> Vec<u8> {
    let physical_level_technique = if config.use_advanced_encoding_schemes {
        PhysicalLevelTechnique::FastPfor
    } else {
        PhysicalLevelTechnique::Varint
    };

    // let maplibre_tile_buffer: Vec<u8> = Vec::new();
    let feature_table_id = 0;
    for layer in mvt.layers {
        let feature_table_name = layer.name;
        // let mvt_features = layer.features;

        let _feature_table_metadata = tileset_metadata.feature_tables.get(feature_table_id);
        let feature_table_optimizations = config.optimizations.get(&feature_table_name);
        sort_features_and_encode_geometry_column(
            config,
            feature_table_optimizations,
            &layer.features,
            physical_level_technique,
        );
    }

    vec![]
}

fn sort_features_and_encode_geometry_column(
    config: &ConversionConfig,
    feature_table_optimizations: Option<&FeatureTableOptimizations>,
    mvt_features: &[Feature],
    _physical_level_technique: PhysicalLevelTechnique,
) {
    let is_column_sortable =
        config.include_ids && feature_table_optimizations.is_some_and(|opt| opt.allow_sorting);

    let mut sorted_features = mvt_features.to_vec();

    if is_column_sortable
        && feature_table_optimizations.is_some_and(|opt| !opt.allow_id_regeneration)
    {
        sorted_features.sort_by_key(|f| f.id);
    }

    let ids: Vec<i64> = sorted_features.iter().map(|f| f.id).collect();
    let _sort_settings = SortSettings::new(is_column_sortable, feature_table_optimizations, ids);

    // Unnecessary cloning: fix later
    let _geometries: Vec<Geometry> = sorted_features.iter().map(|f| f.geometry.clone()).collect();
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
        assert_eq!(metadata.feature_tables.len(), tile.layers.len());
    }
}
