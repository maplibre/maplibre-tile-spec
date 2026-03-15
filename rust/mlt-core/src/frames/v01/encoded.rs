use std::io;
use std::io::Write;

use integer_encoding::VarIntWriter as _;
use utils::BinarySerializer as _;

use crate::v01::{EncodedGeometry, EncodedLayer01};
use crate::{MltError, utils};

impl EncodedLayer01 {
    /// Write layer's binary representation to a [`Write`] stream without allocating a Vec.
    pub fn write_to(&self, writer: &mut impl Write) -> io::Result<()> {
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

    fn write_id_meta(&self, writer: &mut impl Write) -> Result<(), MltError> {
        if let Some(ref id) = self.id {
            id.write_columns_meta_to(writer)?;
        }
        Ok(())
    }

    fn write_id(&self, writer: &mut impl Write) -> Result<(), MltError> {
        if let Some(ref id) = self.id {
            id.write_to(writer)?;
        }
        Ok(())
    }

    #[cfg(not(fuzzing))]
    fn write_columns_meta_to(&self, writer: &mut impl Write) -> Result<(), MltError> {
        self.write_id_meta(writer)?;
        EncodedGeometry::write_columns_meta_to(writer)?;
        for prop in &self.properties {
            prop.write_columns_meta_to(writer)?;
        }
        Ok(())
    }

    /// TODO: force item ordering to be stable in the spec
    #[cfg(fuzzing)]
    fn write_columns_meta_to(&self, writer: &mut impl Write) -> Result<(), MltError> {
        use root::LayerOrdering;
        let props = &mut self.properties.iter();
        for ord in &self.layer_order {
            match ord {
                LayerOrdering::Id => self.write_id_meta(writer)?,
                LayerOrdering::Geometry => EncodedGeometry::write_columns_meta_to(writer)?,
                LayerOrdering::Property => {
                    let prop = props.next().expect("layer order count mismatch");
                    prop.write_columns_meta_to(writer)?;
                }
            }
        }
        Ok(())
    }

    #[cfg(not(fuzzing))]
    fn write_columns_to(&self, writer: &mut impl Write) -> Result<(), MltError> {
        self.write_id(writer)?;
        self.geometry.write_to(writer)?;
        for prop in &self.properties {
            prop.write_to(writer)?;
        }
        Ok(())
    }

    /// TODO: force item ordering to be stable in the spec
    #[cfg(fuzzing)]
    fn write_columns_to(&self, writer: &mut impl Write) -> Result<(), MltError> {
        use root::LayerOrdering;
        let props = &mut self.properties.iter();
        for ord in &self.layer_order {
            match ord {
                LayerOrdering::Id => self.write_id(writer)?,
                LayerOrdering::Geometry => self.geometry.write_to(writer)?,
                LayerOrdering::Property => {
                    let prop = props.next().expect("layer order count mismatch");
                    prop.write_to(writer)?;
                }
            }
        }
        Ok(())
    }
}
