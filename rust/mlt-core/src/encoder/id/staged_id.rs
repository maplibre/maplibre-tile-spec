use crate::MltResult;
use crate::decoder::ColumnType;
use crate::encoder::optimizer::{Presence, PropertyStats};
use crate::encoder::{
    Codecs, Encoder, StagedOptScalar, StagedScalar, write_opt_u32_scalar_col,
    write_opt_u64_scalar_col, write_u32_scalar_col, write_u64_scalar_col,
};

/// Staged ID column (encode-side, fully owned).
///
/// Mirrors the `StagedProperty` enum pattern but without a column name: the
/// staged value itself carries the eventual ID column width and optionality.
#[derive(Debug, Clone, PartialEq)]
pub enum StagedId {
    None,
    U32(StagedScalar<u32>),
    OptU32(StagedOptScalar<u32>),
    U64(StagedScalar<u64>),
    OptU64(StagedOptScalar<u64>),
}

impl StagedId {
    /// Construct from a sparse `Vec<Option<u64>>`.
    ///
    /// Width is selected from the maximum present value; any `None` produces an
    /// optional variant with a dense values vector.
    #[must_use]
    pub fn from_optional(ids: Vec<Option<u64>>) -> Self {
        let len = ids.len();
        let mut values = Vec::with_capacity(len);
        let mut values64: Option<Vec<u64>> = None;
        let mut presence: Option<Vec<bool>> = None;

        for (idx, id) in ids.into_iter().enumerate() {
            match id {
                Some(id) => {
                    if let Some(values64) = &mut values64 {
                        values64.push(id);
                    } else if let Ok(id) = u32::try_from(id) {
                        values.push(id);
                    } else {
                        let mut promoted = Vec::with_capacity(len);
                        promoted.extend(values.iter().copied().map(u64::from));
                        promoted.push(id);
                        values64 = Some(promoted);
                    }
                    if let Some(presence) = &mut presence {
                        presence.push(true);
                    }
                }
                None => {
                    presence
                        .get_or_insert_with(|| {
                            let mut presence = Vec::with_capacity(len);
                            presence.resize(idx, true);
                            presence
                        })
                        .push(false);
                }
            }
        }

        if values.is_empty() && values64.is_none() {
            Self::None
        } else if let Some(presence) = presence {
            if let Some(values64) = values64 {
                Self::OptU64(StagedOptScalar::from_parts(
                    String::new(),
                    presence,
                    values64,
                ))
            } else {
                Self::OptU32(StagedOptScalar::from_parts(String::new(), presence, values))
            }
        } else if let Some(values64) = values64 {
            Self::u64(values64)
        } else {
            Self::u32(values)
        }
    }

    /// Construct from sparse IDs using a precomputed presence classification.
    #[must_use]
    pub(crate) fn from_optional_with_presence(
        ids: impl IntoIterator<Item = Option<u64>>,
        analysis: Option<&PropertyStats>,
    ) -> Self {
        let Some(analysis) = analysis else {
            return Self::None;
        };
        match analysis.presence {
            Presence::AllNull => Self::None,
            Presence::AllPresent => {
                Self::from_dense(ids.into_iter().flatten(), analysis.stats.values_fit_u32())
            }
            Presence::Mixed => Self::from_optional_sparse(ids, analysis.stats.values_fit_u32()),
        }
    }

    fn from_dense(ids: impl IntoIterator<Item = u64>, values_fit_u32: bool) -> Self {
        if values_fit_u32 {
            Self::u32(
                ids.into_iter()
                    .map(|id| {
                        u32::try_from(id).expect("ID analysis guarantees u32-compatible values")
                    })
                    .collect(),
            )
        } else {
            Self::u64(ids.into_iter().collect())
        }
    }

    fn from_optional_sparse(
        ids: impl IntoIterator<Item = Option<u64>>,
        values_fit_u32: bool,
    ) -> Self {
        let ids = ids.into_iter();
        let (lower, upper) = ids.size_hint();
        let capacity = upper.unwrap_or(lower);
        let mut presence = Vec::with_capacity(capacity);
        if values_fit_u32 {
            let mut values = Vec::with_capacity(capacity);
            for id in ids {
                presence.push(id.is_some());
                if let Some(id) = id {
                    values.push(
                        u32::try_from(id).expect("ID analysis guarantees u32-compatible values"),
                    );
                }
            }
            Self::OptU32(StagedOptScalar::from_parts(String::new(), presence, values))
        } else {
            let mut values = Vec::with_capacity(capacity);
            for id in ids {
                presence.push(id.is_some());
                if let Some(id) = id {
                    values.push(id);
                }
            }
            Self::OptU64(StagedOptScalar::from_parts(String::new(), presence, values))
        }
    }

    #[must_use]
    pub fn u32(values: Vec<u32>) -> Self {
        Self::U32(StagedScalar {
            name: String::new(),
            values,
        })
    }

    #[must_use]
    pub fn opt_u32(values: impl IntoIterator<Item = Option<u32>>) -> Self {
        Self::OptU32(StagedOptScalar::from_optional(String::new(), values))
    }

    #[must_use]
    pub fn u64(values: Vec<u64>) -> Self {
        Self::U64(StagedScalar {
            name: String::new(),
            values,
        })
    }

    #[must_use]
    pub fn opt_u64(values: impl IntoIterator<Item = Option<u64>>) -> Self {
        Self::OptU64(StagedOptScalar::from_optional(String::new(), values))
    }

    /// Encode and write the ID column to `enc`.
    #[hotpath::measure]
    pub fn write_to(self, enc: &mut Encoder, codecs: &mut Codecs) -> MltResult<()> {
        match &self {
            Self::None => return Ok(()),
            Self::U32(v) => write_u32_scalar_col(ColumnType::Id, None, v, enc, codecs)?,
            Self::OptU32(v) => {
                write_opt_u32_scalar_col(ColumnType::OptId, None, v, enc, codecs)?;
            }
            Self::U64(v) => write_u64_scalar_col(ColumnType::LongId, None, v, enc, codecs)?,
            Self::OptU64(v) => {
                write_opt_u64_scalar_col(ColumnType::OptLongId, None, v, enc, codecs)?;
            }
        }

        enc.increment_column_count();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use geo_types::Point;
    use proptest::prelude::*;
    use rstest::rstest;

    use crate::decoder::{ColumnType as CT, GeometryValues, RawId, RawIdValue};
    use crate::encoder::model::CurveParams;
    use crate::encoder::stream::LogicalEncoder;
    use crate::encoder::{
        Codecs, Encoder, EncoderConfig, ExplicitEncoder, IntEncoder, StagedId, StagedLayer,
    };
    use crate::test_helpers::{dec, into_layer01, parser};
    use crate::{Layer, LazyParsed, MltError, MltResult};

    /// Round-trip `StagedId` via full layer bytes using an explicit encoder.
    fn id_roundtrip_via_layer(decoded: &StagedId, int_enc: IntEncoder) -> StagedId {
        if matches!(decoded, StagedId::None) {
            return StagedId::from_optional(vec![]);
        }
        let n = feature_count(decoded);
        let mut geometry = GeometryValues::default();
        for _ in 0..n {
            geometry.push_geom(&geo_types::Geometry::<i32>::Point(Point::new(0, 0)));
        }
        let staged = StagedLayer {
            name: "id_roundtrip".to_string(),
            extent: 4096,
            id: decoded.clone(),
            geometry,
            properties: vec![],
            curve_params: CurveParams::default(),
        };
        let enc = Encoder::with_explicit(Encoder::default().cfg, ExplicitEncoder::for_id(int_enc));
        let mut codecs = Codecs::default();
        let enc = staged.encode_into(enc, &mut codecs).expect("encode failed");
        let buf = enc.into_layer_bytes().expect("into_layer_bytes failed");
        let mut p = parser();
        let (_, layer) = Layer::from_bytes(&buf, &mut p).expect("parse failed");

        let parsed = into_layer01(layer)
            .id
            .expect("expected id column")
            .into_parsed(&mut dec())
            .expect("decode failed");
        StagedId::from_optional(parsed.materialize())
    }

    fn id_roundtrip_auto(decoded: &StagedId) -> StagedId {
        if matches!(decoded, StagedId::None) {
            return StagedId::from_optional(vec![]);
        }
        let n = feature_count(decoded);
        let mut geometry = GeometryValues::default();
        for _ in 0..n {
            geometry.push_geom(&geo_types::Geometry::<i32>::Point(Point::new(0, 0)));
        }
        let staged = StagedLayer {
            name: "id_roundtrip".to_string(),
            extent: 4096,
            id: decoded.clone(),
            geometry,
            properties: vec![],
            curve_params: CurveParams::default(),
        };
        let mut codecs = Codecs::default();
        let buf = staged
            .encode_into(Encoder::default(), &mut codecs)
            .expect("encode failed")
            .into_layer_bytes()
            .expect("into_layer_bytes failed");
        let mut p = parser();
        let mut d = dec();
        let (_, layer) = Layer::from_bytes(&buf, &mut p).expect("parse failed");
        assert!(p.reserved() > 0, "parser should reserve bytes after parse");
        let layer01 = into_layer01(layer);

        let result = layer01
            .id
            .expect("expected id column")
            .into_parsed(&mut d)
            .expect("decode failed");
        assert!(
            d.consumed() > 0,
            "decoder should consume bytes after decode"
        );
        StagedId::from_optional(result.materialize())
    }

    fn create_u32_range_ids() -> StagedId {
        StagedId::from_optional((1u64..=100).map(Some).collect())
    }

    fn create_u64_range_ids() -> StagedId {
        let base = u64::from(u32::MAX) + 1;
        StagedId::from_optional((base..base + 50).map(Some).collect())
    }

    fn create_ids_with_nulls() -> StagedId {
        StagedId::from_optional(vec![Some(10), None, Some(20), None, Some(30)])
    }

    fn create_constant_ids() -> StagedId {
        StagedId::from_optional(vec![Some(42), Some(42), Some(42), Some(42), Some(42)])
    }

    fn feature_count(ids: &StagedId) -> usize {
        match ids {
            StagedId::None => 0,
            StagedId::U32(v) => v.values.len(),
            StagedId::OptU32(v) => v.presence.len(),
            StagedId::U64(v) => v.values.len(),
            StagedId::OptU64(v) => v.presence.len(),
        }
    }

    fn id_values(ids: &StagedId) -> Vec<Option<u64>> {
        match ids {
            StagedId::None => Vec::new(),
            StagedId::U32(v) => v.values.iter().copied().map(u64::from).map(Some).collect(),
            StagedId::OptU32(v) => {
                let mut dense = v.values.iter().copied().map(u64::from);
                v.presence
                    .iter()
                    .map(|&p| if p { dense.next() } else { None })
                    .collect()
            }
            StagedId::U64(v) => v.values.iter().copied().map(Some).collect(),
            StagedId::OptU64(v) => {
                let mut dense = v.values.iter().copied();
                v.presence
                    .iter()
                    .map(|&p| if p { dense.next() } else { None })
                    .collect()
            }
        }
    }

    /// Verify that automatic encoding produces no column for empty or all-null ID lists.
    #[rstest]
    #[case::empty(StagedId::from_optional(vec![]))]
    #[case::all_nulls(StagedId::from_optional(vec![None, None]))]
    fn test_automatic_encoding_skipped(#[case] input: StagedId) {
        let mut enc = Encoder::default();
        let mut codecs = Codecs::default();
        input.write_to(&mut enc, &mut codecs).unwrap();
        assert_eq!(
            enc.layer_column_count, 0,
            "empty or all-null ID list should write no column"
        );
    }

    /// Verify that automatic encoding produces a column for non-trivial inputs.
    #[rstest]
    #[case::short_sequence(StagedId::from_optional(vec![Some(1), Some(2)]))]
    #[case::sequential_u32(create_u32_range_ids())]
    #[case::sequential_u64(create_u64_range_ids())]
    #[case::constant(create_constant_ids())]
    #[case::with_nulls(create_ids_with_nulls())]
    fn test_automatic_encoding_produces_output(#[case] input: StagedId) {
        let mut enc = Encoder::default();
        let mut codecs = Codecs::default();
        input.write_to(&mut enc, &mut codecs).unwrap();
        assert_eq!(
            enc.layer_column_count, 1,
            "non-trivial ID list should write a column"
        );
    }

    #[test]
    fn test_automatic_optimization_roundtrip_empty() {
        let decoded = StagedId::from_optional(vec![]);
        let mut enc = Encoder::default();
        let mut codecs = Codecs::default();
        decoded.write_to(&mut enc, &mut codecs).unwrap();
        assert_eq!(
            enc.layer_column_count, 0,
            "empty ID list should write no column"
        );
    }

    #[rstest]
    #[case::sequential_u32(create_u32_range_ids())]
    #[case::sequential_u64(create_u64_range_ids())]
    #[case::constant(create_constant_ids())]
    #[case::with_nulls(create_ids_with_nulls())]
    fn test_automatic_optimization_roundtrip(#[case] decoded: StagedId) {
        let decoded_back = id_roundtrip_auto(&decoded);
        assert_eq!(decoded_back, decoded);
    }

    #[test]
    fn test_manual_optimization_applies_encoder() {
        let decoded = StagedId::u64((1..=100).collect());
        let decoded_back =
            id_roundtrip_via_layer(&decoded, IntEncoder::varint_with(LogicalEncoder::None));
        assert_eq!(id_values(&decoded_back), id_values(&decoded));
    }

    #[test]
    fn test_manual_u32_roundtrip() {
        let ids = StagedId::u32(vec![41]);
        let decoded_back =
            id_roundtrip_via_layer(&ids, IntEncoder::varint_with(LogicalEncoder::None));
        assert_eq!(id_values(&decoded_back), vec![Some(41)]);
    }

    #[test]
    fn test_manual_fastpfor_roundtrip() {
        let ids = StagedId::u32((0u32..200).map(|i| i * 7 + 3).collect());
        let decoded_back = id_roundtrip_via_layer(&ids, IntEncoder::fastpfor());
        assert_eq!(decoded_back, ids);
    }

    /// Verify that large sequential u32 IDs produce a smaller encoding than plain varint
    /// (confirming that delta+FastPFOR is being selected automatically).
    #[test]
    fn test_auto_fastpfor_beats_varint_for_large_u32_ids() {
        let ids = StagedId::from_optional((0u64..1000).map(|i| Some(i * 13 + 5)).collect());

        let mut auto_enc = Encoder::default();
        let mut codecs = Codecs::default();
        ids.clone().write_to(&mut auto_enc, &mut codecs).unwrap();

        let mut plain_enc = Encoder::with_explicit(
            Encoder::default().cfg,
            ExplicitEncoder::for_id(IntEncoder::varint()),
        );
        ids.write_to(&mut plain_enc, &mut codecs).unwrap();

        assert!(
            auto_enc.total_len() <= plain_enc.total_len(),
            "auto ({} bytes) should not be worse than plain varint ({} bytes)",
            auto_enc.total_len(),
            plain_enc.total_len()
        );
    }

    /// Verify that large u64 IDs round-trip correctly under automatic encoding.
    #[test]
    fn test_auto_roundtrip_large_u64_ids() {
        let base = u64::from(u32::MAX) + 1;
        let ids = StagedId::from_optional((0u64..1000).map(|i| Some(base + i * 13)).collect());
        let decoded_back = id_roundtrip_auto(&ids);
        assert_eq!(decoded_back, ids);
    }

    // Test that each config produces the correct variant and optional stream presence
    #[rstest]
    #[case::id32(CT::Id, StagedId::u32(vec![1, 2, 3]))]
    #[case::empty_id32(CT::Id, StagedId::u32(vec![]))]
    #[case::opt_id32(CT::OptId, StagedId::opt_u32(vec![Some(1), None, Some(3)]))]
    #[case::id64(CT::LongId, StagedId::u64(vec![1, 2, 3]))]
    #[case::opt_id64(CT::OptLongId, StagedId::opt_u64(vec![Some(1), None, Some(3)]))]
    fn test_config_produces_correct_variant(#[case] column_type: CT, #[case] input: StagedId) {
        let int_enc = IntEncoder::varint_with(LogicalEncoder::None);
        with_encoded_raw_id(&input, int_enc, |raw_id| {
            match column_type {
                CT::OptId | CT::Id => assert!(matches!(raw_id.value, RawIdValue::Id32(_))),
                CT::LongId | CT::OptLongId => assert!(matches!(raw_id.value, RawIdValue::Id64(_))),
                _ => unreachable!(),
            }
            match column_type {
                CT::OptId | CT::OptLongId => assert!(raw_id.presence.0.is_some()),
                CT::Id | CT::LongId => assert!(raw_id.presence.0.is_none()),
                _ => unreachable!(),
            }
        });
    }

    #[rstest]
    #[case::id32_basic(StagedId::u32(vec![1, 2, 100, 1000]))]
    #[case::id32_single(StagedId::u32(vec![42]))]
    #[case::id32_boundaries(StagedId::u32(vec![0, u32::MAX]))]
    #[case::id64_basic(StagedId::u64(vec![1, 2, 100, 1000]))]
    #[case::id64_single(StagedId::u64(vec![u64::MAX]))]
    #[case::id64_boundaries(StagedId::u64(vec![0, u64::MAX]))]
    #[case::id64_large_values(StagedId::u64(vec![0, u64::from(u32::MAX), u64::from(u32::MAX) + 1, u64::MAX]))]
    #[case::opt_id32_with_nulls(StagedId::opt_u32(vec![Some(1), None, Some(100), None, Some(1000)]))]
    #[case::opt_id32_no_nulls(StagedId::opt_u32(vec![Some(1), Some(2), Some(3)]))]
    #[case::opt_id32_single_null(StagedId::opt_u32(vec![None]))]
    #[case::opt_id64_with_nulls(StagedId::opt_u64(vec![Some(1), None, Some(u64::from(u32::MAX) + 1), None, Some(u64::MAX)]))]
    #[case::opt_id64_all_nulls(StagedId::opt_u64(vec![None, None, None]))]
    #[case::none(StagedId::None)]
    #[case::empty_id32(StagedId::u32(vec![]))]
    fn test_roundtrip(#[case] ids: StagedId) {
        let int_enc = IntEncoder::varint_with(LogicalEncoder::None);
        assert_roundtrip(&ids, int_enc);
    }

    #[rstest]
    fn test_sequential_ids(
        #[values(LogicalEncoder::None)] logical: LogicalEncoder,
        #[values(CT::Id, CT::OptId, CT::LongId, CT::OptLongId)] column_type: CT,
    ) {
        let input = match column_type {
            CT::Id => StagedId::u32((1..=100).collect()),
            CT::OptId => StagedId::opt_u32((1..=100).map(Some)),
            CT::LongId => StagedId::u64((1..=100).collect()),
            CT::OptLongId => StagedId::opt_u64((1..=100).map(Some)),
            _ => unreachable!(),
        };
        let int_enc = IntEncoder::varint_with(logical);
        assert_roundtrip(&input, int_enc);
    }

    proptest! {
        #[test]
        fn test_roundtrip_opt_id32(
            ids in prop::collection::vec(prop::option::of(any::<u32>()), 1..100),
            logical in any::<LogicalEncoder>()
        ) {
            prop_assert_roundtrip(&StagedId::opt_u32(ids), IntEncoder::varint_with(logical))?;
        }

        #[test]
        fn test_roundtrip_id64(
            ids in prop::collection::vec(any::<u64>(), 1..100),
            logical in any::<LogicalEncoder>()
        ) {
            prop_assert_roundtrip(&StagedId::u64(ids), IntEncoder::varint_with(logical))?;
        }

        #[test]
        fn test_roundtrip_id32(
            ids in prop::collection::vec(any::<u32>(), 1..100),
            logical in any::<LogicalEncoder>()
        ) {
            prop_assert_roundtrip(&StagedId::u32(ids), IntEncoder::varint_with(logical))?;
        }

        #[test]
        fn test_roundtrip_opt_id64(
            ids in prop::collection::vec(prop::option::of(any::<u64>()), 1..100),
            logical in any::<LogicalEncoder>()
        ) {
            prop_assert_roundtrip(&StagedId::opt_u64(ids), IntEncoder::varint_with(logical))?;
        }

        #[test]
        fn test_correct_variant_produced_id32(
            ids in prop::collection::vec(1u32..1000u32, 1..50),
            logical in any::<LogicalEncoder>()
        ) {
            assert_produces_correct_variant(&StagedId::u32(ids), CT::Id, IntEncoder::varint_with(logical))?;
        }

        #[test]
        fn test_correct_variant_produced_id64(
            ids in prop::collection::vec(any::<u64>(), 1..50),
            logical in any::<LogicalEncoder>()
        ) {
            assert_produces_correct_variant(&StagedId::u64(ids), CT::LongId, IntEncoder::varint_with(logical))?;
        }
    }

    /// Round-trip `StagedId` via full layer bytes (encode → bytes → parse → decode).
    fn assert_roundtrip(ids: &StagedId, int_enc: IntEncoder) {
        prop_assert_roundtrip(ids, int_enc).expect("roundtrip failed");
    }

    fn prop_assert_roundtrip(ids: &StagedId, int_enc: IntEncoder) -> Result<(), TestCaseError> {
        let res = roundtrip_id_values(ids, int_enc)
            .map_err(|e| TestCaseError::Fail(format!("Roundtrip failed: {e:?}").into()))?;
        let expected = StagedId::from_optional(id_values(ids));
        prop_assert_eq!(id_values(&res), id_values(&expected));
        Ok(())
    }

    fn roundtrip_id_values(decoded: &StagedId, int_enc: IntEncoder) -> MltResult<StagedId> {
        if matches!(decoded, StagedId::None) {
            return Ok(StagedId::from_optional(vec![]));
        }
        let n = feature_count(decoded);
        let mut geometry = GeometryValues::default();
        for _ in 0..n {
            geometry.push_geom(&geo_types::Geometry::<i32>::Point(Point::new(0, 0)));
        }
        let staged = StagedLayer {
            name: "id_roundtrip".to_string(),
            extent: 4096,
            id: decoded.clone(),
            geometry,
            properties: vec![],
            curve_params: CurveParams::default(),
        };
        let enc =
            Encoder::with_explicit(EncoderConfig::default(), ExplicitEncoder::for_id(int_enc));
        let mut codecs = Codecs::default();
        let enc = staged.encode_into(enc, &mut codecs)?;
        let buf = enc.into_layer_bytes()?;
        let (_, layer) = Layer::from_bytes(&buf, &mut parser())?;
        let Layer::Tag01(layer01) = layer else {
            return Err(MltError::NotDecoded("expected Tag01 layer"));
        };
        // When all source IDs were null, the encoder skips the ID column entirely.
        // On decode, the absent column is semantically identical to all-null IDs.
        match layer01.id {
            Some(id) => {
                let parsed = id.into_parsed(&mut dec())?;
                Ok(StagedId::from_optional(parsed.materialize()))
            }
            None => Ok(StagedId::from_optional(vec![None; n])),
        }
    }

    /// Encode `ids` into a full layer, parse the raw ID, and pass it to `f`.
    fn with_encoded_raw_id<R>(
        ids: &StagedId,
        int_enc: IntEncoder,
        f: impl FnOnce(&RawId<'_>) -> R,
    ) -> R {
        // Encode a full StagedLayer, parse the layer back out, and inspect the raw ID field.
        // This exercises the ID encoding as it appears in a real layer payload.
        let n = feature_count(ids);
        let mut geometry = GeometryValues::default();
        for _ in 0..n {
            geometry.push_geom(&geo_types::Geometry::<i32>::Point(Point::new(0, 0)));
        }
        let staged = StagedLayer {
            name: "id_test".to_string(),
            extent: 4096,
            id: ids.clone(),
            geometry,
            properties: vec![],
            curve_params: CurveParams::default(),
        };
        let enc =
            Encoder::with_explicit(EncoderConfig::default(), ExplicitEncoder::for_id(int_enc));
        let mut codecs = Codecs::default();
        let enc = staged.encode_into(enc, &mut codecs).expect("encode failed");
        let buf = enc.into_layer_bytes().expect("into_layer_bytes failed");
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).expect("parse failed");
        let Layer::Tag01(layer01) = layer else {
            panic!("expected Tag01")
        };
        let Some(LazyParsed::Raw(raw_id)) = layer01.id else {
            panic!("expected raw id")
        };
        f(&raw_id)
    }

    fn assert_produces_correct_variant(
        input: &StagedId,
        column_type: CT,
        int_enc: IntEncoder,
    ) -> Result<(), TestCaseError> {
        with_encoded_raw_id(input, int_enc, |raw_id| {
            if matches!(column_type, CT::Id | CT::OptId) {
                prop_assert!(
                    matches!(raw_id.value, RawIdValue::Id32(_)),
                    "Expected Id32 variant"
                );
            } else {
                prop_assert!(
                    matches!(raw_id.value, RawIdValue::Id64(_)),
                    "Expected Id64 variant"
                );
            }

            if matches!(column_type, CT::OptId | CT::OptLongId) {
                prop_assert!(
                    raw_id.presence.0.is_some(),
                    "Expected optional stream to be present"
                );
            } else {
                prop_assert!(raw_id.presence.0.is_none(), "Expected no optional stream");
            }
            Ok(())
        })
    }
}
