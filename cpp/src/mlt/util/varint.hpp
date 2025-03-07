#pragma once

#include <mlt/common.hpp>
#include <mlt/util/buffer_stream.hpp>

#include <stdexcept>
#include <type_traits>

namespace mlt::util::decoding {

template <typename T>
    requires(std::is_integral_v<T>)
T decodeVarint(BufferStream&);

template <>
inline std::uint32_t decodeVarint(BufferStream& buffer) {
    // Max 4 bytes supported
    auto b = buffer.read<std::uint8_t>();
    auto value = static_cast<std::uint32_t>(b) & 0x7f;
    if (b & 0x80) {
        b = buffer.read<std::uint8_t>();
        value |= static_cast<std::uint32_t>(b & 0x7f) << 7;
        if (b & 0x80) {
            b = buffer.read<std::uint8_t>();
            value |= static_cast<std::uint32_t>(b & 0x7f) << 14;
            if (b & 0x80) {
                value |= (buffer.read<std::uint8_t>() & 0x7f) << 21;
            }
        }
    }
    return value;
}

template <>
inline std::uint64_t decodeVarint(BufferStream& buffer) {
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
    requires(std::is_integral_v<T> || std::is_enum_v<T>, 0 < N)
auto decodeVarints(BufferStream& buffer) {
    auto v = std::make_tuple(static_cast<T>(decodeVarint<std::uint32_t>(buffer)));
    if constexpr (N == 1) {
        return v;
    } else
        return std::tuple_cat(std::move(v), decodeVarints<T, N - 1>(buffer));
}

/// Decode N varints into the provided buffer
/// Each result is cast to the target type.
template <typename TDecode, typename TTarget = TDecode>
    requires(std::is_integral_v<TDecode> && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>) &&
             sizeof(TDecode) <= sizeof(TTarget))
void decodeVarints(BufferStream& buffer, const count_t numValues, TTarget* out) {
    std::generate_n(out, numValues, [&buffer]() { return static_cast<TTarget>(decodeVarint<TDecode>(buffer)); });
}

} // namespace mlt::util::decoding
