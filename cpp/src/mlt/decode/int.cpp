#include <mlt/decode/int.hpp>
#include <mlt/polyfill.hpp> // NOLINT(misc-include-cleaner)
#include <mlt/util/buffer_stream.hpp>

#include <fastpfor/compositecodec.h>
#include <fastpfor/fastpfor.h>
#include <fastpfor/variablebyte.h>

#include <algorithm>
#include <bit>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <stdexcept>

namespace mlt::decoder {

struct IntegerDecoder::Impl {
    FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte> codec;
};

IntegerDecoder::IntegerDecoder([[maybe_unused]] bool enableFastPFOR)
    : impl(std::make_unique<Impl>()),
      enableFastPFOR(enableFastPFOR) {}

IntegerDecoder::~IntegerDecoder() noexcept = default;

std::uint32_t IntegerDecoder::decodeFastPfor([[maybe_unused]] BufferStream& buffer,
                                             [[maybe_unused]] std::uint32_t* const result,
                                             [[maybe_unused]] const std::size_t numValues,
                                             [[maybe_unused]] const std::size_t byteLength) {
    if (enableFastPFOR) {
        const auto* inputValues = reinterpret_cast<const std::uint32_t*>(buffer.getReadPosition());

        // TODO: change to little endian in the encoder?
        const auto intLength = (byteLength + sizeof(std::uint32_t) - 1) / sizeof(std::uint32_t);
        const auto leBuffer = getTempBuffer<std::uint32_t>(intLength);
        std::transform(inputValues, inputValues + intLength, leBuffer.get(), [](std::uint32_t v) noexcept {
            return std::byteswap(v);
        });

        auto resultCount = numValues;
        impl->codec.decodeArray(leBuffer, intLength, result, resultCount);
        buffer.consume(byteLength);
        return static_cast<std::uint32_t>(resultCount);
    }
    throw std::runtime_error("FastPFOR decoding is not enabled");
}

} // namespace mlt::decoder
