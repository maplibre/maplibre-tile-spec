#[allow(
    unused_imports,
    clippy::wildcard_imports,
    reason = "not worth for fuzzing"
)]
use crate::v01::*;

#[cfg(fuzzing)]
/// To make sure we serialize out in the same order as the original file, we need to store the order in which we parsed the columns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum LayerOrdering {
    Id,
    Geometry,
    Property,
}

#[cfg(fuzzing)]
impl From<ColumnType> for LayerOrdering {
    fn from(typ: ColumnType) -> Self {
        use crate::frames::v01::model::ColumnType::*;
        match typ {
            OptId | Id | LongId | OptLongId => Self::Id,
            Bool | OptBool | I8 | OptI8 | U8 | OptU8 | I32 | OptI32 | U32 | OptU32 | I64
            | OptI64 | U64 | OptU64 | F32 | OptF32 | F64 | OptF64 | Str | OptStr | SharedDict => {
                Self::Property
            }
            Geometry => Self::Geometry,
        }
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for EncodedLayer01 {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let name: String = u.arbitrary()?;
        let extent: u32 = u.arbitrary()?;
        let id: Option<crate::encoder::EncodedId> = if u.arbitrary()? {
            Some(u.arbitrary()?)
        } else {
            None
        };
        let geometry = u.arbitrary()?;
        let properties: Vec<EncodedProperty> = u.arbitrary()?;

        #[cfg(fuzzing)]
        let layer_order = {
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
            layer_order
        };

        Ok(Self {
            name,
            extent,
            id,
            geometry,
            properties,
            #[cfg(fuzzing)]
            layer_order,
        })
    }
}
