#include <util/rle.hpp>

#include <metadata/stream.hpp>

#include <cassert>

namespace mlt::util::decoding::rle {

void decodeBoolean(BufferStream& buffer,
                   std::vector<uint8_t>& out,
                   const metadata::stream::StreamMetadata& metadata,
                   bool consume) noexcept(false) {
    const auto bitCount = metadata.getNumValues();
    const auto numBytes = (bitCount + 7) / 8;
    out.resize(numBytes);

    assert(metadata.getByteLength() <= buffer.getRemaining());
    detail::ByteRleDecoder decoder{buffer.getReadPosition(),
                                   std::min<std::size_t>(metadata.getByteLength(), buffer.getRemaining())};
    decoder.next(out.data(), numBytes);
    assert(decoder.getBufferRemaining() == 0);

    if (consume) {
        buffer.consume(metadata.getByteLength());
    }
}

} // namespace mlt::util::decoding::rle
