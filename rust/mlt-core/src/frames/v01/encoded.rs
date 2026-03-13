use std::io;
use std::io::Write;

use integer_encoding::VarIntWriter as _;
use utils::BinarySerializer as _;

use crate::v01::EncodedLayer01;
use crate::{MltError, utils};

impl EncodedLayer01 {
    /// Write layer's binary representation to a [`Write`] stream without allocating a Vec.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_string(&self.name)?;
        writer.write_varint(self.extent)?;

        let id_columns_count = u32::from(self.id.is_some());
        let geometry_column_count = 1u32;
        let property_column_count = u32::try_from(self.properties.len()).map_err(MltError::from)?;
        let column_count = property_column_count + id_columns_count + geometry_column_count;
        writer.write_varint(column_count)?;

        let map_error_to_io = |e: MltError| match e {
            MltError::Io(e) => e,
            e => io::Error::other(e),
        };
        self.write_columns_meta_to(writer)
            .map_err(map_error_to_io)?;
        self.write_columns_to(writer).map_err(map_error_to_io)?;
        Ok(())
    }

    #[cfg(not(fuzzing))]
    fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        if let Some(ref id) = self.id {
            id.write_columns_meta_to(writer)?;
        }
        crate::v01::EncodedGeometry::write_columns_meta_to(writer)?;
        for prop in &self.properties {
            prop.write_columns_meta_to(writer)?;
        }
        Ok(())
    }

    #[cfg(fuzzing)]
    fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        use crate::v01::root::LayerOrdering;
        let props = &mut self.properties.iter();
        for ord in &self.layer_order {
            match ord {
                LayerOrdering::Id => {
                    if let Some(ref id) = self.id {
                        id.write_columns_meta_to(writer)?;
                    }
                }
                LayerOrdering::Geometry => {
                    crate::v01::EncodedGeometry::write_columns_meta_to(writer)?;
                }
                LayerOrdering::Property => {
                    let prop = props.next().expect(
                        "the number of layer order elements must match the number of properties",
                    );
                    prop.write_columns_meta_to(writer)?;
                }
            }
        }
        Ok(())
    }

    #[cfg(not(fuzzing))]
    fn write_columns_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        if let Some(ref id) = self.id {
            id.write_to(writer)?;
        }
        self.geometry.write_to(writer)?;
        for prop in &self.properties {
            prop.write_to(writer)?;
        }
        Ok(())
    }

    #[cfg(fuzzing)]
    fn write_columns_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        use crate::v01::root::LayerOrdering;
        let props = &mut self.properties.iter();
        for ord in &self.layer_order {
            match ord {
                LayerOrdering::Id => {
                    if let Some(ref id) = self.id {
                        id.write_to(writer)?;
                    }
                }
                LayerOrdering::Geometry => self.geometry.write_to(writer)?,
                LayerOrdering::Property => {
                    let prop = props.next().expect(
                        "the number of layer order elements must match the number of properties",
                    );
                    prop.write_to(writer)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(all(fuzzing, feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for EncodedLayer01 {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        use crate::v01::root::LayerOrdering;
        use crate::v01::{EncodedId, EncodedProperty};

        let name: String = u.arbitrary()?;
        let extent: u32 = u.arbitrary()?;
        let id: Option<EncodedId> = if u.arbitrary()? {
            Some(u.arbitrary()?)
        } else {
            None
        };
        let geometry = u.arbitrary()?;
        let properties: Vec<EncodedProperty> = u.arbitrary()?;

        // Build a valid layer_order and Fisher-Yates shuffle it.
        let mut layer_order: Vec<LayerOrdering> = Vec::new();
        if id.is_some() {
            layer_order.push(LayerOrdering::Id);
        }
        layer_order.push(LayerOrdering::Geometry);
        for _ in &properties {
            layer_order.push(LayerOrdering::Property);
        }
        let n = layer_order.len();
        for i in (1..n).rev() {
            let j: usize = u.int_in_range(0..=i)?;
            layer_order.swap(i, j);
        }

        Ok(EncodedLayer01 {
            name,
            extent,
            id,
            geometry,
            properties,
            layer_order,
        })
    }
}
