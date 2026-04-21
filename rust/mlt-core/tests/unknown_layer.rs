/// Integration tests for [`mlt_core::Unknown`] public accessors.
///
/// Builds a minimal hand-crafted byte buffer that contains one unknown layer
/// (tag = 42, body = [1, 2, 3]) and verifies that `Parser::parse_layers` yields
/// a `Layer::Unknown` whose `tag()` and `data()` return the expected values.
///
/// Wire format of a single layer:
/// ```text
///   [varint: body_len_including_tag_byte] [tag_byte] [body_bytes...]
///
///   For tag=42, body=[1,2,3]:
///     body_len = 1 (tag byte) + 3 (body bytes) = 4
///     bytes = [0x04, 0x2A, 0x01, 0x02, 0x03]
/// ```
use mlt_core::{Layer, Parser};

fn unknown_layer_bytes(tag: u8, body: &[u8]) -> Vec<u8> {
    // size varint = 1 (tag byte) + body.len()
    let size = (1 + body.len()) as u64;

    let mut buf = Vec::new();
    // encode size as varint
    let mut v = size;
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            buf.push(byte);
            break;
        }
        buf.push(byte | 0x80);
    }
    buf.push(tag);
    buf.extend_from_slice(body);
    buf
}

#[test]
fn unknown_tag_and_data_are_accessible() {
    let body: &[u8] = &[1, 2, 3];
    let raw = unknown_layer_bytes(42, body);

    let layers = Parser::default()
        .parse_layers(&raw)
        .expect("parse should succeed");

    assert_eq!(layers.len(), 1);

    let Layer::Unknown(u) = &layers[0] else {
        panic!("expected Layer::Unknown, got {:?}", layers[0]);
    };

    assert_eq!(u.tag(), 42u32);
    assert_eq!(u.data(), body);
}

#[test]
fn unknown_zero_length_body() {
    let raw = unknown_layer_bytes(99, &[]);

    let layers = Parser::default()
        .parse_layers(&raw)
        .expect("parse should succeed");

    let Layer::Unknown(u) = &layers[0] else {
        panic!("expected Layer::Unknown");
    };

    assert_eq!(u.tag(), 99u32);
    assert!(u.data().is_empty());
}

#[test]
fn multiple_layers_mixed_unknown_and_tag01() {
    // Build two unknown layers back-to-back. Tags 1 and 2 are known (Tag01 and Tag02),
    // so use tags 10 and 11 for these "unknown" tests.
    let mut raw = unknown_layer_bytes(10, b"hello");
    raw.extend_from_slice(&unknown_layer_bytes(11, b"world"));

    let layers = Parser::default()
        .parse_layers(&raw)
        .expect("parse should succeed");

    assert_eq!(layers.len(), 2);

    let Layer::Unknown(u0) = &layers[0] else {
        panic!("expected Unknown at index 0");
    };
    assert_eq!(u0.tag(), 10u32);
    assert_eq!(u0.data(), b"hello");

    let Layer::Unknown(u1) = &layers[1] else {
        panic!("expected Unknown at index 1");
    };
    assert_eq!(u1.tag(), 11u32);
    assert_eq!(u1.data(), b"world");
}
