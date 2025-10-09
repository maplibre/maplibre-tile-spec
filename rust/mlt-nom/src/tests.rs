use crate::structures::{Layer, Unknown};
use crate::utils::parse_varint;

#[test]
fn test_varint_parsing() {
    // Test single byte varint
    assert_eq!(parse_varint(&[0x00]), Ok((&[][..], 0)));
    assert_eq!(parse_varint(&[0x7F]), Ok((&[][..], 127)));

    // Test multi-byte varint
    assert_eq!(parse_varint(&[0x80, 0x01]), Ok((&[][..], 128)));
    assert_eq!(
        parse_varint(&[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]),
        Ok((&[][..], 0xFFFF_FFFF_u32))
    );
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
