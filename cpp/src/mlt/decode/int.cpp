#include <mlt/decode/int.hpp>
#include <mlt/decode/int_template.hpp>

// from fastpfor/...
#include <compositecodec.h>
#include <fastpfor.h>
#include <variablebyte.h>

#include <cstdint>

namespace mlt::decoder {

struct IntegerDecoder::Impl {
    // Prevent FastPFOR from being an external dependency
    FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte> codec;
};

IntegerDecoder::IntegerDecoder()
    : impl(std::make_unique<Impl>()) {}

IntegerDecoder::~IntegerDecoder() noexcept = default;

std::uint32_t IntegerDecoder::decodeFastPfor(BufferStream& buffer,
                                             std::uint32_t* const result,
                                             const std::size_t numValues,
                                             const std::size_t byteLength) {
    const auto* inputValues = reinterpret_cast<const std::uint32_t*>(buffer.getReadPosition());
    auto resultCount = numValues;
    impl->codec.decodeArray(inputValues, byteLength / sizeof(std::uint32_t), result, resultCount);
    return static_cast<std::uint32_t>(resultCount);
}

} // namespace mlt::decoder
