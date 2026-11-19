#include <mlt/util/rle.hpp>

#include <mlt/metadata/stream.hpp>
#include <mlt/util/buffer_stream.hpp>

#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <vector>

namespace mlt::util::decoding::rle {

void decodeBoolean(BufferStream& tileData,
                   std::vector<uint8_t>& out,
                   const metadata::stream::StreamMetadata& metadata,
                   bool consume) {
    const auto bitCount = metadata.getNumValues();
    const std::size_t numBytes = (static_cast<std::size_t>(bitCount) + 7) / 8;

    if (metadata.getByteLength() > tileData.getRemaining()) {
        throw std::runtime_error("Unexpected end of buffer");
    }
    // Each pair of encoded bytes can describe at most 130 bytes, so check the length before allocating
    if (numBytes / 65 > metadata.getByteLength()) {
        throw std::runtime_error("Boolean stream has more values than its data can hold");
    }
    out.resize(numBytes);

    detail::ByteRleDecoder decoder{tileData.getReadPosition(), metadata.getByteLength()};
    decoder.next(out.data(), numBytes);

    if (consume) {
        tileData.consume(metadata.getByteLength());
    }
}

} // namespace mlt::util::decoding::rle
