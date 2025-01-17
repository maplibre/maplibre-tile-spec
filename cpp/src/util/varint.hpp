#pragma once

#include <common.hpp>

#include <stdexcept>

namespace mlt::util::decoding {

inline std::int32_t decodeVarint(DataView src, offset_t& offset) noexcept {
    // Max 4 bytes supported.
    auto b = src[offset++];
    auto value = b & 0x7f;
    if (b & 0x80) {
        b = src[offset++];
        value |= (b & 0x7f) << 7;
        if (b & 0x80) {
            b = src[offset++];
            value |= (b & 0x7f) << 14;
            if (b & 0x80) {
                value |= (src[offset++] & 0x7f) << 21;
            }
        }
    }
    return value;
}

inline std::int64_t decodeLongVarint(DataView bytes, offset_t& pos) {
    std::int64_t value = 0;
    offset_t index = pos;
    for (int shift = 0; index < bytes.size(); ) {
        auto b = bytes[index++];
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

    pos = index;
    return value;
}

/// Decode N varints, retrurning the values in a `std::tuple`
template <std::size_t N>
auto decodeVarints(DataView src, offset_t& pos) noexcept;

// TODO: variadic template magic so each one doesn't have to be explicit
template <>
inline auto decodeVarints<1ul>(DataView src, offset_t& pos) noexcept {
    return decodeVarint(src, pos);
}
template <>
inline auto decodeVarints<2ul>(DataView src, offset_t& pos) noexcept {
    return std::make_tuple(decodeVarint(src, pos), decodeVarint(src, pos));
}
template <>
inline auto decodeVarints<3ul>(DataView src, offset_t& pos) noexcept {
    return std::make_tuple(decodeVarint(src, pos), decodeVarint(src, pos), decodeVarint(src, pos));
}
template <>
inline auto decodeVarints<4ul>(DataView src, offset_t& pos) noexcept {
    return std::make_tuple(
        decodeVarint(src, pos), decodeVarint(src, pos), decodeVarint(src, pos), decodeVarint(src, pos));
}

/// Decode N varints into the by-reference arguments
template <typename... Ts>
void decodeVarints(DataView src, offset_t& pos, Ts&... values) noexcept {
    (([&] { values = decodeVarint(src, pos); })(), ...);
}

template <typename... Ts>
void decodeLongVarints(DataView src, offset_t& pos, Ts&... values) {
    (decodeLongVarint(src, pos, values), ...);
}

inline void decodeVarint(DataView src, offset_t& pos, int numValues, std::int32_t* output) noexcept {
    offset_t dstOffset = 0;
    for (int i = 0; i < numValues; ++i) {
        output[dstOffset++] = decodeVarint(src, pos);
    }
}

inline void decodeLongVarint(DataView src, offset_t& pos, int numValues, std::int64_t* output) {
    offset_t dstOffset = 0;
    for (int i = 0; i < numValues; ++i) {
        output[dstOffset++] = decodeLongVarint(src, pos);
    }
}

}
