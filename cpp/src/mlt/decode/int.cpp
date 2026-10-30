#include <mlt/decode/fastpfor.hpp>
#include <mlt/decode/int.hpp>
#include <mlt/polyfill.hpp> // NOLINT(misc-include-cleaner)
#include <mlt/util/buffer_stream.hpp>

#include <bit>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <span>
#include <stdexcept>

namespace mlt::decoder {

void IntegerDecoder::decodeFastPfor(BufferStream& buffer,
                                    std::uint32_t* const result,
                                    const std::size_t numValues,
                                    const std::size_t byteLength) {
    if (byteLength % sizeof(std::uint32_t) != 0) {
        throw std::runtime_error("FastPFOR stream length is not a multiple of 4 bytes");
    }

    // The length comes from the tile, so check it against the data before reading
    const auto* const inputBytes = buffer.getReadPosition();
    buffer.consume(byteLength);

    // TODO: change to little endian in the encoder?
    const auto intLength = byteLength / sizeof(std::uint32_t);
    const auto leBuffer = getTempBuffer<std::uint32_t>(intLength);
    const std::span<std::uint32_t> words{leBuffer.get(), intLength};
    for (std::size_t i = 0; i < intLength; ++i) {
        // The input is not necessarily aligned
        std::uint32_t value = 0;
        std::memcpy(&value, inputBytes + (i * sizeof(value)), sizeof(value));
        words[i] = std::byteswap(value);
    }

    fastpfor::decode(words, {result, numValues});
}

} // namespace mlt::decoder
