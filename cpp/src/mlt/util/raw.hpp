#pragma once

#include <mlt/common.hpp>
#include <mlt/metadata/stream.hpp>

#include <algorithm>
#include <cassert>

namespace mlt::util::decoding {

inline void decodeRaw(BufferStream& buffer, std::vector<std::uint8_t>& out, count_t numBytes, bool consume) {
    out.resize(numBytes);
    std::copy(buffer.getReadPosition(), buffer.getReadPosition() + numBytes, out.begin());
    if (consume) {
        buffer.consume(numBytes);
    }
}

template <typename T>
void decodeRaw(BufferStream& buffer,
               std::vector<T>& out,
               const metadata::stream::StreamMetadata& metadata,
               bool consume) {
    const auto numValues = metadata.getNumValues();
    const auto numBytes = sizeof(T) * numValues;
    assert(numBytes == metadata.getByteLength());
    out.resize(numValues);
    std::copy(
        buffer.getReadPosition(), buffer.getReadPosition() + numBytes, reinterpret_cast<std::uint8_t*>(out.data()));
    if (consume) {
        buffer.consume(numBytes);
    }
}

} // namespace mlt::util::decoding
