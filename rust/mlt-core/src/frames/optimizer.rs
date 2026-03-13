// Layer-level optimizer impls have been removed along with StagedLayer.
// Encoding is now done directly on StagedLayer01 via:
//   StagedLayer01::encode(encoder) -> Result<EncodedLayer01, MltError>
//   StagedLayer01::encode_with_profile(profile) -> Result<(EncodedLayer01, Tag01Encoder), MltError>
//   StagedLayer01::encode_automatic() -> Result<(EncodedLayer01, Tag01Encoder), MltError>
