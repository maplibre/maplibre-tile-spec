#pragma once

#include <common.hpp>
#include <util/buffer_stream.hpp>

#include <stdexcept>
#include <vector>

namespace mlt::util::decoding {

template <typename T>
    requires(std::is_integral_v<T>)
T decodeVarint(BufferStream&) noexcept(false);

template <>
inline std::uint32_t decodeVarint(BufferStream& buffer) noexcept(false) {
    // Max 4 bytes supported
    auto b = static_cast<std::uint32_t>(buffer.read());
    auto value = static_cast<std::uint32_t>(b) & 0x7f;
    if (b & 0x80) {
        b = buffer.read();
        value |= (b & 0x7f) << 7;
        if (b & 0x80) {
            b = buffer.read();
            value |= (b & 0x7f) << 14;
            if (b & 0x80) {
                value |= (buffer.read() & 0x7f) << 21;
            }
        }
    }
    return value;
}

template <>
inline std::uint64_t decodeVarint(BufferStream& buffer) noexcept(false) {
    std::uint64_t value = 0;
    for (int shift = 0; buffer.available();) {
        auto b = static_cast<std::uint32_t>(buffer.read());
        value |= (long)(b & 0x7F) << shift;
        if ((b & 0x80) == 0) {
            break;
        }

        shift += 7;
        if (shift >= 64) {
            // TODO: do we really want exceptions?
            throw std::runtime_error("Varint too long");
        }
    }
    return value;
}

/// Decode N varints, retrurning the values in a `std::tuple`
template <typename T, std::size_t N>
    requires(std::is_integral_v<T>, 0 < N)
auto decodeVarints(BufferStream& buffer) noexcept(false) {
    auto v = std::make_tuple(decodeVarint<std::uint32_t>(buffer));
    if constexpr (N == 1) {
        return v;
    } else
        return std::tuple_cat(std::move(v), decodeVarints<T, N - 1>(buffer));
}

/// Decode N varints into the provided buffer
/// Each result is cast to the target type.
template <typename TDecode, typename TTarget = TDecode>
    requires(std::is_integral_v<TDecode> && std::is_integral_v<TTarget> && sizeof(TDecode) <= sizeof(TTarget))
void decodeVarints(BufferStream& buffer, count_t numValues, TTarget* out) noexcept(false) {
    std::generate_n(out, numValues, [&buffer]() { return static_cast<TTarget>(decodeVarint<TDecode>(buffer)); });
}

template <typename TDecode, typename TTarget = TDecode>
    requires(std::is_integral_v<TDecode> && std::is_integral_v<TTarget> && sizeof(TDecode) <= sizeof(TTarget))
void decodeVarints(BufferStream& buffer, std::vector<TTarget>& out) noexcept(false) {
    decodeVarints<TDecode, TTarget>(buffer, out.size(), out.data());
}

} // namespace mlt::util::decoding
