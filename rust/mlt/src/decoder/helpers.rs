use crate::metadata::proto_tileset::{Column, ScalarType, column, scalar_column};
use crate::{MltError, MltResult};

/// Get the physical scalarType from a Column metadata.
pub fn get_data_type_from_column(column_metadata: &Column) -> MltResult<ScalarType> {
    match column_metadata.r#type.as_ref() {
        Some(column::Type::ScalarType(scalar_column)) => match scalar_column.r#type {
            Some(scalar_column::Type::PhysicalType(scalar_type)) => {
                ScalarType::try_from(scalar_type)
                    .map_err(|_| MltError::MetaDecodeInvalidType("ScalarType"))
            }
            Some(_) => Err(MltError::MetaDecodeUnsupportedType("column.scalar.type")),
            None => Err(MltError::MissingField("column.scalar.type")),
        },
        Some(_) => Err(MltError::MetaDecodeUnsupportedType("column.type")),
        None => Err(MltError::MissingField("column.type")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::proto_tileset::ScalarColumn;

    #[test]
    fn test_get_data_type_from_column() {
        let column_metadata = Column {
            name: "id".to_string(),
            nullable: false,
            column_scope: 0,
            r#type: Some(column::Type::ScalarType(ScalarColumn {
                long_id: false,
                r#type: Some(scalar_column::Type::PhysicalType(ScalarType::Uint32 as i32)),
            })),
        };
        let data_type =
            get_data_type_from_column(&column_metadata).expect("should parse ScalarType");
        assert_eq!(data_type, ScalarType::Uint32);
    }
}
