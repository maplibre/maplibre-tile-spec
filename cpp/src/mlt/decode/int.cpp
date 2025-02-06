#include <mlt/decode/int.hpp>
#include <mlt/decode/int_template.hpp>

// from fastpfor/...
#include <compositecodec.h>
#include <fastpfor.h>
#include <variablebyte.h>

#include <cstdint>

namespace mlt::decoder {

struct IntegerDecoder::Impl {
    FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte> codec;
    // Reused storage for intermediate values
    std::vector<std::uint32_t> buffer4;
    std::vector<std::uint64_t> buffer8;
};

IntegerDecoder::IntegerDecoder() noexcept(false)
    : impl(std::make_unique<Impl>()) {}

IntegerDecoder::~IntegerDecoder() noexcept = default;

template <typename T>
std::vector<T>& IntegerDecoder::getTempBuffer() noexcept {
    // Can we make these covariant and eliminate the casts?
    if constexpr (std::is_trivially_assignable_v<decltype(impl->buffer4)::reference, T>) {
        return reinterpret_cast<std::vector<T>&>(impl->buffer4);
    } else if constexpr (std::is_trivially_assignable_v<decltype(impl->buffer8)::reference, T>) {
        return static_cast<std::vector<T>&>(impl->buffer8);
    }
    // Ideally this would be a `static_assert`, but that fails even if never reached by template expansion.
    assert(false);
    auto* fail = static_cast<std::vector<T>*>(nullptr);
    return *fail;
}
template std::vector<std::int32_t>& IntegerDecoder::getTempBuffer<std::int32_t>() noexcept;
template std::vector<std::uint32_t>& IntegerDecoder::getTempBuffer<std::uint32_t>() noexcept;
template std::vector<std::uint64_t>& IntegerDecoder::getTempBuffer<std::uint64_t>() noexcept;

std::uint32_t IntegerDecoder::decodeFastPfor(BufferStream& buffer,
                                             std::uint32_t* const result,
                                             const std::size_t numValues,
                                             const std::size_t byteLength) noexcept(false) {
    const auto* inputValues = reinterpret_cast<const std::uint32_t*>(buffer.getReadPosition());
    auto resultCount = numValues;
    impl->codec.decodeArray(inputValues, byteLength / sizeof(std::uint32_t), result, resultCount);
    return resultCount;
}

} // namespace mlt::decoder
