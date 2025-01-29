#pragma once

#include <common.hpp>
#include <metadata/stream.hpp>

#include <cassert>

namespace mlt::util::decoding {

inline void decodeRaw(BufferStream& buffer,
                      std::vector<std::uint8_t>& out,
                      count_t numBytes,
                      bool consume) noexcept(false) {
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
               bool consume) noexcept(false) {
    const auto numValues = metadata.getNumValues();
    const auto numBytes = sizeof(T) * numValues;
    assert(numBytes == metadata.getByteLength());
    out.resize(numValues);
    std::memcpy(out.data(), buffer.getReadPosition(), numBytes);
    if (consume) {
        buffer.consume(numBytes);
    }
}

} // namespace mlt::util::decoding
