#pragma once

#include <mlt/common.hpp>
#include <mlt/metadata/stream.hpp>

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <vector>

namespace mlt::util::decoding {

inline void decodeRaw(BufferStream& tileData, std::vector<std::uint8_t>& out, std::uint32_t numBytes, bool consume) {
    // The length comes from the tile, so check that the data is there before allocating for it or reading it
    if (!tileData.available(numBytes)) {
        throw std::runtime_error("Unexpected end of buffer");
    }
    out.resize(numBytes);
    std::copy_n(tileData.getReadPosition(), numBytes, out.begin());
    if (consume) {
        tileData.consume(numBytes);
    }
}

template <typename T>
void decodeRaw(BufferStream& tileData,
               std::vector<T>& out,
               const metadata::stream::StreamMetadata& metadata,
               bool consume) {
    const auto numValues = metadata.getNumValues();
    const auto numBytes = sizeof(T) * static_cast<std::size_t>(numValues);
    if (numBytes != metadata.getByteLength()) {
        throw std::runtime_error("unexpected byte length for raw stream");
    }
    if (!tileData.available(numBytes)) {
        throw std::runtime_error("Unexpected end of buffer");
    }
    out.resize(numValues);
    std::copy_n(tileData.getReadPosition(), numBytes, reinterpret_cast<std::uint8_t*>(out.data()));
    if (consume) {
        tileData.consume(numBytes);
    }
}

} // namespace mlt::util::decoding
