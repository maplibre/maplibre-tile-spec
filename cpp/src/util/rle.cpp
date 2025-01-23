#include <util/rle.hpp>

#include <metadata/stream.hpp>

namespace mlt::util::decoding::rle {

void decodeBoolean(BufferStream& buffer,
                   std::vector<uint8_t>& out,
                   const metadata::stream::StreamMetadata& metadata,
                   bool consume) {
    const auto bitCount = metadata.getNumValues();
    const auto numBytes = (bitCount + 7) / 8;
    out.resize(numBytes);
    detail::ByteRleDecoder decoder{buffer.getReadPosition(), buffer.getRemaining()};
    decoder.next(out.data(), numBytes);
    if (consume) {
        buffer.consume(numBytes);
    }
}

} // namespace mlt::util::decoding::rle
