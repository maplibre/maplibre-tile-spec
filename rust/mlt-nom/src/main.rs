use nom::Err::Error as NomError;
use nom::bytes::complete::take;
use nom::combinator::complete;
use nom::error::{Error, ErrorKind};
use nom::multi::many0;
use nom::{IResult, Parser};
use num_enum::TryFromPrimitive;

mod parsers;

/// A layer that can be either MVT-compatible or unknown
#[derive(Debug, PartialEq)]
pub enum Layer<'a> {
    /// MVT-compatible layer (tag = 1)
    LayerV1(LayerV1<'a>),
    /// Unknown layer with tag, size, and value
    Unknown(Unknown<'a>),
}

impl Layer<'_> {
    /// Parse a single binary tuple: size (varint), tag (varint), value (bytes)
    pub fn parse(input: &[u8]) -> IResult<&[u8], Layer<'_>> {
        let (input, size) = parsers::parse_varint_usize(input)?;

        // tag is a varint, but we know fewer than 127 tags for now,
        // so we can use a faster u8 and fail if it is bigger than 127.
        let (input, tag) = parsers::parse_u8(input)?;
        // 1 byte must be parsed for the tag, so if size is 0, it's invalid
        let size = size
            .checked_sub(1)
            .ok_or(NomError(Error::new(input, ErrorKind::Fail)))?;
        let (input, value) = take(size)(input)?;

        let layer = match tag {
            1 => {
                let (data, meta) = LayerMeta::parse(value)?;
                LayerV1::parse(data, meta)?
            }
            tag => Layer::Unknown(Unknown { tag, value }),
        };

        Ok((input, layer))
    }
}

/// MVT-compatible layer data
#[derive(Debug, PartialEq)]
pub struct LayerV1<'a> {
    pub meta: LayerMeta<'a>,
    pub data: &'a [u8],
}

impl LayerV1<'_> {
    #[expect(clippy::unnecessary_wraps)]
    pub fn parse<'a>(
        input: &'a [u8],
        meta: LayerMeta<'a>,
    ) -> Result<Layer<'a>, nom::Err<Error<&'a [u8]>>> {
        for column in &meta.columns {
            #[expect(clippy::match_same_arms)]
            match column.typ {
                ColumnType::Id => {
                    // TODO: parse id
                }
                ColumnType::Geometry => {
                    // TODO
                }
                ColumnType::StringProperty => {
                    // TODO
                }
                ColumnType::FloatProperty => {
                    // TODO
                }
                ColumnType::DoubleProperty => {
                    // TODO
                }
                ColumnType::IntProperty => {
                    // TODO
                }
                ColumnType::UintProperty => {
                    // TODO
                }
                ColumnType::SintProperty => {
                    // TODO
                }
                ColumnType::BoolProperty => {
                    // TODO
                }
            }
        }

        Ok(Layer::LayerV1(LayerV1 { meta, data: input }))
    }
}

/// Layer V1 metadata structure
#[derive(Debug, PartialEq)]
pub struct LayerMeta<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub columns: Vec<Column<'a>>,
}

impl LayerMeta<'_> {
    /// Parse Layer V1 metadata
    pub fn parse(input: &[u8]) -> IResult<&[u8], LayerMeta<'_>> {
        let (input, name) = parsers::parse_string(input)?;
        let (input, extent) = parsers::parse_varint_u32(input)?;
        let (mut input, column_count) = parsers::parse_varint_usize(input)?;

        let mut columns = Vec::with_capacity(column_count);
        for _ in 0..column_count {
            let pair = Column::parse(input)?;
            input = pair.0;
            columns.push(pair.1);
        }

        Ok((
            input,
            LayerMeta {
                name,
                extent,
                columns,
            },
        ))
    }
}

/// Column definition
#[derive(Debug, PartialEq)]
pub struct Column<'a> {
    pub typ: ColumnType,
    pub name: Option<&'a str>,
}

impl Column<'_> {
    /// Parse a single column definition
    fn parse(input: &[u8]) -> IResult<&[u8], Column<'_>> {
        let (mut input, typ) = ColumnType::parse(input)?;
        let name = if typ != ColumnType::Id && typ != ColumnType::Geometry {
            let pair = parsers::parse_string(input)?;
            input = pair.0;
            Some(pair.1)
        } else {
            None
        };
        Ok((input, Column { typ, name }))
    }
}

/// Column type enumeration
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum ColumnType {
    Id = 1,
    // TODO: decide if we need additional geometry types like
    //   PointGeometry/LineGeometry/PolygonGeometry -- if all of them are the same
    //   PreTessellated geometry - if we include additional tessellation data
    Geometry = 2,
    StringProperty = 3,
    FloatProperty = 4,
    DoubleProperty = 5,
    IntProperty = 6,
    UintProperty = 7,
    SintProperty = 8,
    BoolProperty = 9,
}

impl ColumnType {
    /// Parse a column type from u8
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, value) = parsers::parse_u8(input)?;
        let value = Self::try_from(value);
        let value = value.or(Err(NomError(Error::new(input, ErrorKind::Fail))))?;
        Ok((input, value))
    }
}

/// Unknown layer data
#[derive(Debug, PartialEq)]
pub struct Unknown<'a> {
    pub tag: u8,
    pub value: &'a [u8],
}

/// Parse a sequence of binary layers
pub fn parse_binary_stream(input: &[u8]) -> IResult<&[u8], Vec<Layer<'_>>> {
    many0(complete(Layer::parse)).parse(input)
}

fn main() {
    println!("\n=== Layer Stream Parsing Demo ===");
    let test_data = create_test_data();
    println!("Parsing test data of size: {}", test_data.len());

    match parse_binary_stream(&test_data) {
        Ok((remaining, layers)) => {
            println!("Successfully parsed {} layers:", layers.len());
            for (i, layer) in layers.iter().enumerate() {
                match layer {
                    Layer::LayerV1(layer_v1) => {
                        println!("  Layer {i}: LayerV1");
                        println!("    Name: {}", layer_v1.meta.name);
                        println!("    Extent: {}", layer_v1.meta.extent);
                        for (j, column) in layer_v1.meta.columns.iter().enumerate() {
                            match column.name {
                                Some(name) => {
                                    println!(
                                        "    Column {j}: name='{name}', type={:?}",
                                        column.typ
                                    );
                                }
                                None => println!("    Column {j}: type={:?}", column.typ),
                            }
                        }
                        println!("    Data: {:?}", layer_v1.data);
                    }
                    Layer::Unknown(unknown) => {
                        println!(
                            "  Layer {i}: Unknown(tag={}), value={:?}",
                            unknown.tag, unknown.value
                        );
                    }
                }
            }
            if !remaining.is_empty() {
                println!("Warning: {} bytes remaining unparsed", remaining.len());
            }
        }
        Err(e) => println!("Parse error: {e:?}"),
    }
}

/// Helper function to create test data
fn create_test_data() -> Vec<u8> {
    let mut data = Vec::new();

    // Tuple 1: tag=1 (LayerV1), create metadata + data
    let mut layer = Vec::new();

    // Layer name: "roads" (length=5, then "roads")
    parsers::encode_str(&mut layer, b"roads");

    // Extent: 4096
    parsers::encode_varint(&mut layer, 4096);

    // Column count: 6
    parsers::encode_varint(&mut layer, 6);

    // Column 1: type=1 (ID) - no name for ID columns
    parsers::encode_varint(&mut layer, 1); // column_type (ID)

    // Column 2: type=2 (Geometry) - no name for Geometry columns
    parsers::encode_varint(&mut layer, 2); // column_type (Geometry)

    // Column 3: type=3 (StringProperty) - has name
    parsers::encode_varint(&mut layer, 3); // column_type (StringProperty)
    parsers::encode_str(&mut layer, b"name"); // name

    // Column 4: type=4 (FloatProperty) - has name
    parsers::encode_varint(&mut layer, 4); // column_type (FloatProperty)
    parsers::encode_str(&mut layer, b"price"); // name

    // Column 5: type=6 (IntProperty) - has name
    parsers::encode_varint(&mut layer, 6); // column_type (IntProperty)
    parsers::encode_str(&mut layer, b"count"); // name

    // Column 6: type=9 (BoolProperty) - has name
    parsers::encode_varint(&mut layer, 9); // column_type (BoolProperty)
    parsers::encode_str(&mut layer, b"active"); // name

    // Add some additional data after metadata
    layer.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]);

    parsers::encode_varint(&mut data, layer.len() as u64 + 1); // size (1 for tag + data)
    parsers::encode_varint(&mut data, 1); // tag
    data.extend_from_slice(&layer); // value

    // Tuple 2: tag=42, size=3 (1 for tag + 2 for value), value=[0xAA, 0xBB]
    parsers::encode_varint(&mut data, 3); // size (1 for tag + 2 for value)
    parsers::encode_varint(&mut data, 42); // tag
    data.extend_from_slice(&[0xAA, 0xBB]); // value

    // Tuple 3: tag=100, size=0, value=[]
    parsers::encode_varint(&mut data, 1); // size (1 for tag byte)
    parsers::encode_varint(&mut data, 100); // tag
    // no value bytes for size 0

    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::{encode_str, encode_varint, parse_varint};

    #[test]
    fn test_varint_parsing() {
        // Test single byte varint
        assert_eq!(parse_varint(&[0x00]), Ok((&[][..], 0)));
        assert_eq!(parse_varint(&[0x7F]), Ok((&[][..], 127)));

        // Test multi-byte varint
        assert_eq!(parse_varint(&[0x80, 0x01]), Ok((&[][..], 128)));
        assert_eq!(
            parse_varint(&[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]),
            Ok((&[][..], 0xFFFF_FFFF))
        );
    }

    #[test]
    fn test_tuple_parsing() {
        // Create test data with metadata + data for LayerV1
        let mut data = Vec::new();

        let mut layer = Vec::new();
        // Layer name: "roads" (length=5, then "roads")
        encode_str(&mut layer, b"roads");

        // Extent: 4096
        encode_varint(&mut layer, 4096);

        // Column count: 6
        encode_varint(&mut layer, 6);

        // Column 1: type=1 (ID) - no name for ID columns
        encode_varint(&mut layer, 1); // column_type (Id)

        // Column 2: type=2 (Geometry) - no name for Geometry columns
        encode_varint(&mut layer, 2); // column_type (Geometry)

        // Column 3: type=3 (StringProperty) - has name
        encode_varint(&mut layer, 3); // column_type (StringProperty)
        encode_str(&mut layer, b"name"); // name

        // Column 4: type=4 (FloatProperty) - has name
        encode_varint(&mut layer, 4); // column_type (FloatProperty)
        encode_str(&mut layer, b"price"); // name

        // Column 5: type=6 (IntProperty) - has name
        encode_varint(&mut layer, 6); // column_type (IntProperty)
        encode_str(&mut layer, b"count"); // name

        // Column 6: type=9 (BoolProperty) - has name
        encode_varint(&mut layer, 9); // column_type (BoolProperty)
        encode_str(&mut layer, b"active"); // name

        // Add some additional data after metadata
        layer.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]);

        encode_varint(&mut data, layer.len() as u64 + 1); // size (1 for tag + data)
        encode_varint(&mut data, 1); // tag
        data.extend_from_slice(&layer); // value

        // Create expected metadata
        let expected_meta = LayerMeta {
            name: "roads",
            extent: 4096,
            columns: vec![
                Column {
                    typ: ColumnType::Id,
                    name: None,
                },
                Column {
                    typ: ColumnType::Geometry,
                    name: None,
                },
                Column {
                    typ: ColumnType::StringProperty,
                    name: Some("name"),
                },
                Column {
                    typ: ColumnType::FloatProperty,
                    name: Some("price"),
                },
                Column {
                    typ: ColumnType::IntProperty,
                    name: Some("count"),
                },
                Column {
                    typ: ColumnType::BoolProperty,
                    name: Some("active"),
                },
            ],
        };

        let expected = Layer::LayerV1(LayerV1 {
            meta: expected_meta,
            data: &[0x01, 0x02, 0x03, 0x04, 0x05],
        });

        println!("Test data: {data:?}");
        let result = Layer::parse(&data);
        println!("Parse result: {result:?}");
        assert_eq!(result, Ok((&[][..], expected)));
    }

    #[test]
    fn test_unknown_tuple_parsing() {
        let data = vec![
            0x04, // size = 4 (1 for tag + 3 for value)
            0x02, // tag = 2 (not 1, so should be Unknown)
            0x01, 0x02, 0x03, // value
        ];

        let expected = Layer::Unknown(Unknown {
            tag: 2,
            value: &[0x01, 0x02, 0x03],
        });

        assert_eq!(Layer::parse(&data), Ok((&[][..], expected)));
    }

    #[test]
    fn test_layer_v1_meta_parsing() {
        // Create test data: name="test_layer", column_count=2, then two columns
        let mut data = Vec::new();

        // Layer name: "test_layer" (length=10, then "test_layer")
        encode_str(&mut data, b"test_layer");

        // Extent: 4096
        encode_varint(&mut data, 4096);

        // Column count: 2
        encode_varint(&mut data, 2);

        // Column 1: type=1 (ID) - no name for ID columns
        encode_varint(&mut data, 1); // column_type (Id)

        // Column 2: type=2 (Geometry) - no name for Geometry columns
        encode_varint(&mut data, 2); // column_type (Geometry)

        let expected = LayerMeta {
            name: "test_layer",
            extent: 4096,
            columns: vec![
                Column {
                    typ: ColumnType::Id,
                    name: None,
                },
                Column {
                    typ: ColumnType::Geometry,
                    name: None,
                },
            ],
        };

        assert_eq!(LayerMeta::parse(&data), Ok((&[][..], expected)));
    }

    #[test]
    fn test_stream_parsing() {
        let data = create_test_data();
        let result = parse_binary_stream(&data);

        assert!(result.is_ok());
        let (remaining, layers) = result.unwrap();
        assert!(remaining.is_empty());
        assert_eq!(layers.len(), 3);

        // Check first layer (tag=1, should be LayerV1)
        match &layers[0] {
            Layer::LayerV1(layer_v1) => {
                assert_eq!(layer_v1.meta.name, "roads");
                assert_eq!(layer_v1.meta.extent, 4096);
                assert_eq!(layer_v1.meta.columns.len(), 6);
                assert_eq!(layer_v1.meta.columns[0].name, None);
                assert_eq!(layer_v1.meta.columns[0].typ, ColumnType::Id);
                assert_eq!(layer_v1.meta.columns[1].name, None);
                assert_eq!(layer_v1.meta.columns[1].typ, ColumnType::Geometry);
                assert_eq!(layer_v1.meta.columns[2].name, Some("name"));
                assert_eq!(layer_v1.meta.columns[2].typ, ColumnType::StringProperty);
                assert_eq!(layer_v1.meta.columns[3].name, Some("price"));
                assert_eq!(layer_v1.meta.columns[3].typ, ColumnType::FloatProperty);
                assert_eq!(layer_v1.meta.columns[4].name, Some("count"));
                assert_eq!(layer_v1.meta.columns[4].typ, ColumnType::IntProperty);
                assert_eq!(layer_v1.meta.columns[5].name, Some("active"));
                assert_eq!(layer_v1.meta.columns[5].typ, ColumnType::BoolProperty);
                assert_eq!(layer_v1.data, &[0x01, 0x02, 0x03, 0x04, 0x05]);
            }
            Layer::Unknown(_) => panic!("Expected LayerV1 layer"),
        }

        // Check second layer (tag=42, should be Unknown)
        match &layers[1] {
            Layer::Unknown(unknown) => {
                assert_eq!(unknown.tag, 42);
                assert_eq!(unknown.value, &[0xAA, 0xBB]);
            }
            Layer::LayerV1(_) => panic!("Expected Unknown layer"),
        }

        // Check third layer (tag=100, should be Unknown)
        match &layers[2] {
            Layer::Unknown(unknown) => {
                assert_eq!(unknown.tag, 100);
                assert_eq!(unknown.value, &[]);
            }
            Layer::LayerV1(_) => panic!("Expected Unknown layer"),
        }
    }
}
