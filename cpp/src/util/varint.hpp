#pragma once

#include <common.hpp>
#include <util/buffer_stream.hpp>

#include <stdexcept>

namespace mlt::util::decoding {

inline std::int32_t decodeVarint(BufferStream& buffer) {
    // Max 4 bytes supported
    auto b = buffer.read();
    auto value = b & 0x7f;
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

inline std::int64_t decodeLongVarint(BufferStream& buffer) {
    std::int64_t value = 0;
    for (int shift = 0; buffer.available(); ) {
        auto b = buffer.read();
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
template <std::size_t N>
auto decodeVarints(BufferStream&);

// TODO: variadic template magic so each one doesn't have to be explicit
template <>
inline auto decodeVarints<1ul>(BufferStream& buffer) {
    return decodeVarint(buffer);
}
template <>
inline auto decodeVarints<2ul>(BufferStream& buffer) {
    return std::make_tuple(decodeVarint(buffer), decodeVarint(buffer));
}
template <>
inline auto decodeVarints<3ul>(BufferStream& buffer) {
    return std::make_tuple(decodeVarint(buffer), decodeVarint(buffer), decodeVarint(buffer));
}
template <>
inline auto decodeVarints<4ul>(BufferStream& buffer) {
    return std::make_tuple(
        decodeVarint(buffer), decodeVarint(buffer), decodeVarint(buffer), decodeVarint(buffer));
}

/// Decode N varints into the by-reference arguments
template <typename... Ts>
void decodeVarints(BufferStream& buffer, Ts&... values) {
    (([&] { values = decodeVarint(buffer); })(), ...);
}

template <typename... Ts>
void decodeLongVarints(BufferStream& buffer, Ts&... values) {
    (decodeLongVarint(buffer, values), ...);
}

inline void decodeVarint(BufferStream& buffer, int numValues, std::int32_t* output) {
    offset_t dstOffset = 0;
    for (int i = 0; i < numValues; ++i) {
        output[dstOffset++] = decodeVarint(buffer);
    }
}

inline void decodeLongVarint(BufferStream& buffer, int numValues, std::int64_t* output) {
    offset_t dstOffset = 0;
    for (int i = 0; i < numValues; ++i) {
        output[dstOffset++] = decodeLongVarint(buffer);
    }
}

}
