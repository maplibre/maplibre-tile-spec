use borrowme::ToOwned as _;
use hex::ToHex as _;
use mlt_core::unknown::OwnedUnknown;
use mlt_core::v01::OwnedLayer01;
use mlt_core::{Layer, OwnedLayer};

#[derive(arbitrary::Arbitrary)]
pub struct LayerInput {
    pub bytes: Vec<u8>,
}
impl LayerInput {
    pub fn fuzz_roundtrip(self) {
        let total_len = self.bytes.len();

        // Try to parse the layer
        let Ok((remaining, layer)) = Layer::parse(&self.bytes) else {
            return;
        };
        if layer.as_layer01().is_none() {
            return; // FIXME: not interesting to debug, but has roundtrippability issues
        }
        if !remaining.is_empty() {
            return; // not interesting to debug
        }
        let consumed_input_bytes_size = total_len - remaining.len();
        let consumed_input = &self.bytes[..consumed_input_bytes_size];

        let owned_layer = layer.to_owned();

        // Write the layer to a buffer
        let mut buffer = Vec::<u8>::with_capacity(consumed_input_bytes_size);
        let Ok(_) = owned_layer.write_to(&mut buffer) else {
            return; // FIXME: implement full layer writes
        };

        // Compare without printing to avoid printing lots of data
        if consumed_input != buffer.as_slice() {
            Self::panic_with_helpful_diff(consumed_input, buffer.as_slice(), owned_layer)
        }
    }

    fn panic_with_helpful_diff(input: &[u8], output: &[u8], parsed_input: OwnedLayer) -> ! {
        Self::print_hex_diff(input, output);
        let (_, out) = Layer::parse(input).unwrap_or_else(|e| {
            panic!("After parsing the input and writing it to disk, it cannot be read again because {e}\n\
            Input parsed to a layer and debug printed:\n{parsed_input:#?}");
        });
        let written_owned = out.to_owned();
        Self::try_panic_if_debug_is_different(&parsed_input, &written_owned);
        if parsed_input == written_owned {
            // this will not be fun to debug :(
            Self::print_corresponding_bytes(written_owned);
            panic!(
                "Parsed input and written output parsed back are equal => some dead bits are ignored in the parser\nInput parsed to a layer and debug printed:\n {parsed_input:#?}"
            );
        } else {
            // this should never trigger, and if it does, this is likely a bug in our debugging format
            println!(
                "input and output are also NOT equal => significant state not debug-printed or not written to disk"
            );
            Self::minimize_inequal_but_debug_equal(&parsed_input, &written_owned)
        }
    }

    /// We try to remove prefixes of serialized things that we know are good
    fn minimize_inequal_but_debug_equal(input: &OwnedLayer, output: &OwnedLayer) -> ! {
        use OwnedLayer as OL;
        match (input, output) {
            (OL::Tag01(input), OL::Tag01(output)) => {
                Self::minimize_layer1_inequal_but_debug_equal(input, output)
            }
            (OL::Unknown(input), OL::Unknown(output)) => {
                Self::minimize_unknown_inequal_but_debug_equal(input, output)
            }
            (OL::Unknown(_), OL::Tag01(_)) | (OL::Tag01(_), OL::Unknown(_)) => {
                unreachable!("mismatched layer types generate different debug output")
            }
        }
    }
    fn minimize_layer1_inequal_but_debug_equal(input: &OwnedLayer01, output: &OwnedLayer01) -> ! {
        let OwnedLayer01 {
            name,
            extent,
            id,
            geometry,
            properties,
            #[cfg(fuzzing)]
            layer_order,
        } = input;
        assert_eq!(*name, output.name, "Layer01 name with different names");
        assert_eq!(
            *extent, output.extent,
            "Layer01 extent with different extents"
        );
        assert_eq!(*id, output.id, "Layer01 id with different ids");
        assert_eq!(
            *geometry, output.geometry,
            "Layer01 with different geometries"
        );
        assert_eq!(
            *properties, output.properties,
            "Layer01 with different properties"
        );
        #[cfg(fuzzing)]
        assert_eq!(
            layer_order, &output.layer_order,
            "Layer01 with different layer order"
        );
        unreachable!("all props are compared equal, but the outer does not compare equal");
    }

    /// If the diff shows up in the debug output, this is very much simpler to debug
    fn try_panic_if_debug_is_different(input: &OwnedLayer, output: &OwnedLayer) {
        pretty_assertions::assert_eq!(
            format!("{input:#?}"),
            format!("{output:#?}"),
            "Which means that input and output are entirely different:"
        );
        // Our debug output is shortening some things for readability, lets try non-alternate
        pretty_assertions::assert_eq!(
            format!("{input:?}"),
            format!("{output:?}"),
            "Which means that input and output are slightly different:"
        );
    }
    fn minimize_unknown_inequal_but_debug_equal(input: &OwnedUnknown, output: &OwnedUnknown) -> ! {
        let OwnedUnknown { value, tag } = input;
        assert_eq!(*tag, output.tag, "Unknow tag with different tags");
        assert_eq!(*value, output.value, "Unknow tag with different values");
        unreachable!("all props are compared equal, but the outer does not compare equal");
    }

    fn print_hex_diff(input: &[u8], output: &[u8]) {
        let input_hex = input.encode_hex::<String>();
        let input_hex = format!("[{input_hex}; {}]", input.len());
        let output_hex = output.encode_hex::<String>();
        let output_hex = format!("[{output_hex}; {}]", output.len());

        let mut diff_arrows_hex = String::new();
        for i in 0..input_hex.len().max(output_hex.len()) {
            match (input_hex.get(i..=i), output_hex.get(i..=i)) {
                (Some(i), Some(o)) => {
                    diff_arrows_hex.push(if i != o { '^' } else { ' ' });
                }
                (None, None) => unreachable!(),
                _ => diff_arrows_hex.push_str(" "),
            }
        }

        println!(
            "Buffer does not match consumed input\n\n\
            {input_hex} <- parsed input\n\
            {output_hex} <- written output\n\
            {diff_arrows_hex}\n",
        )
    }

    fn print_corresponding_bytes(layer: OwnedLayer) {
        println!("DEBUG - Here is what the layer looks like as bytes.");
        println!("IMPORTANT: ordering is arbitrary and does not match MLT");
        match layer {
            OwnedLayer::Tag01(l1) => {
                let OwnedLayer01 {
                    name,
                    extent,
                    id,
                    geometry,
                    properties,
                    #[cfg(fuzzing)]
                    layer_order,
                } = l1;
                println!("layer_order: {layer_order:?}");
                println!(
                    "layer name {name} -> {}",
                    name.as_bytes().encode_hex::<String>()
                );
                println!("layer extent: {extent} -> varint({extent})");
                {
                    println!("layer id: {id:?}",);
                    let mut metadata = Vec::new();
                    id.write_columns_meta_to(&mut metadata).unwrap();
                    println!("\tlayer id metadata: {}", metadata.encode_hex::<String>());
                    let mut data = Vec::new();
                    id.write_to(&mut data).unwrap();
                    println!("\tlayer id data: {}", data.encode_hex::<String>());
                }
                for (i, prop) in properties.iter().enumerate() {
                    println!("{i}. property -> {:?}", prop);
                    let mut metadata = Vec::new();
                    prop.write_columns_meta_to(&mut metadata).unwrap();
                    println!("\tprop metadata: {}", metadata.encode_hex::<String>());
                    let mut buf = Vec::new();
                    prop.write_to(&mut buf).unwrap();
                    println!("\tprop data: {}", buf.encode_hex::<String>());
                }
                {
                    println!("{:?}", geometry);
                    let mut metadata = Vec::new();
                    geometry.write_columns_meta_to(&mut metadata).unwrap();
                    println!("\tprop metadata: {}", metadata.encode_hex::<String>());
                    let mut buf = Vec::new();
                    geometry.write_to(&mut buf).unwrap();
                    println!("\tgeometry data: {}", buf.encode_hex::<String>());
                }
            }
            OwnedLayer::Unknown(u) => {
                println!("tag: {}", u.tag);
                println!("value: {}", u.value.encode_hex::<String>());
            }
        }
    }
}

impl std::fmt::Debug for LayerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Input {{\n\tbytes: [0x{}; {}]\n}}\n",
            self.bytes.encode_hex::<String>(),
            self.bytes.len()
        )?;
        write!(f, "As a layer: {:#?}", Layer::parse(&self.bytes))
    }
}
