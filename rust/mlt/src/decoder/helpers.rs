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
            Some(_) => Err(MltError::MetaDecodeUnsupporteddType("column.scalar.type")),
            None => Err(MltError::MissingField("column.scalar.type")),
        },
        Some(_) => Err(MltError::MetaDecodeUnsupporteddType("column.type")),
        None => Err(MltError::MissingField("column.type")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::integer_stream::decode_componentwise_delta_vec2s;
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

    #[test]
    fn test_decode_componentwise_delta_vec2s() {
        // original Vec2s: [(3, 5), (7, 6), (12, 4)]
        // delta:          [3, 5, 4, 1, 5, -2]
        // ZigZag:         [6, 10, 8, 2, 10, 3]
        let encoded_from_positives: Vec<u32> = vec![6, 10, 8, 2, 10, 3];
        let decoded = decode_componentwise_delta_vec2s::<i32>(&encoded_from_positives).unwrap();
        assert_eq!(decoded, vec![3, 5, 7, 6, 12, 4]);

        // original Vec2s: [(3, 5), (-1, 6), (4, -4)]
        // delta:          [3, 5, -4, 1, 5, -10]
        // ZigZag:         [6, 10, 7, 2, 10, 19]
        let encoded_from_negatives: Vec<u32> = vec![6, 10, 7, 2, 10, 19];
        let decoded = decode_componentwise_delta_vec2s::<i32>(&encoded_from_negatives).unwrap();
        assert_eq!(decoded, vec![3, 5, -1, 6, 4, -4]);
    }
}
