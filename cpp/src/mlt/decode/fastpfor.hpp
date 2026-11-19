#pragma once

#include <cstdint>
#include <span>

namespace mlt::decoder::fastpfor {

/// Decode a FastPFor stream followed by a VariableByte tail, as produced by
/// `FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte>` and its Java counterpart.
///
/// The FastPFOR library itself assumes that its input was produced by its encoder and does not validate it. Tiles are
/// not trustworthy, so this decoder checks every length, offset, count and bit width in the stream against the input
/// and output buffers before using it.
///
/// @param words Stream contents, one element per 32-bit word, in native byte order
/// @param out Output buffer, which the stream must fill exactly
/// @throws std::runtime_error if the stream is malformed, or does not hold exactly `out.size()` values
void decode(std::span<const std::uint32_t> words, std::span<std::uint32_t> out);

} // namespace mlt::decoder::fastpfor
