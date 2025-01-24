#pragma once

#include <type_traits>
#include <util/buffer_stream.hpp>

namespace mlt::metadata::stream {
class StreamMetadata;
}

namespace mlt::util::decoding::rle {

namespace detail {
// Borrowed from https://github.com/apache/orc, `/c++/src/ByteRLE.cc`, `ByteRleDecoderImpl::nextInternal`
// Apache License 2.0
class ByteRleDecoder {
public:
    ByteRleDecoder(const std::uint8_t* buffer, size_t length) noexcept
        : bufferStart(reinterpret_cast<const char*>(buffer)),
          bufferEnd(bufferStart + length) {}

    ByteRleDecoder(const ByteRleDecoder&) = delete;
    ByteRleDecoder(ByteRleDecoder&&) = delete;
    ByteRleDecoder& operator=(const ByteRleDecoder&) = delete;
    ByteRleDecoder& operator=(ByteRleDecoder&&) = delete;

    std::size_t getBufferRemaining() const noexcept { return bufferEnd - bufferStart; }

    void next(std::uint8_t* data,
              std::uint64_t numValues
#if SUPPORT_SKIP_NULL
              ,
              const char* notNull
#endif
              ) noexcept(false) {
        std::uint64_t position = 0;

#if SUPPORT_SKIP_NULL
        // skip over null values
        while (notNull && position < numValues && !notNull[position]) {
            position += 1;
        }
#endif

        while (position < numValues) {
            // if we are out of values, read more
            if (remainingValues == 0) {
                readHeader();
            }
            // how many do we read out of this block?
            size_t count = std::min(static_cast<size_t>(numValues - position), remainingValues);
            uint64_t consumed = 0;
            if (repeating) {
#if SUPPORT_SKIP_NULL
                if (notNull) {
                    for (uint64_t i = 0; i < count; ++i) {
                        if (notNull[position + i]) {
                            data[position + i] = value;
                            consumed += 1;
                        }
                    }
                } else
#endif
                {
                    memset(data + position, value, count);
                    consumed = count;
                }
            } else {
#if SUPPORT_SKIP_NULL
                if (notNull) {
                    for (uint64_t i = 0; i < count; ++i) {
                        if (notNull[position + i]) {
                            data[position + i] = static_cast<char>(readByte());
                            consumed += 1;
                        }
                    }
                } else
#endif
                {
                    uint64_t i = 0;
                    while (i < count) {
                        uint64_t copyBytes = std::min(static_cast<uint64_t>(count - i),
                                                      static_cast<uint64_t>(bufferEnd - bufferStart));
                        memcpy(data + position + i, bufferStart, copyBytes);
                        bufferStart += copyBytes;
                        i += copyBytes;
                    }
                    consumed = count;
                }
            }
            remainingValues -= consumed;
            position += count;

#if SUPPORT_SKIP_NULL
            while (notNull && position < numValues && !notNull[position]) {
                position += 1;
            }
#endif
        }
    }

private:
    char readByte() noexcept(false) {
        if (bufferStart == bufferEnd) {
            throw std::runtime_error("Unexpected end of buffer");
        }
        return *bufferStart++;
    }

    void readHeader() noexcept(false) {
        const char ch = readByte();
        if (ch < 0) {
            remainingValues = static_cast<std::size_t>(-ch);
            repeating = false;
        } else {
            remainingValues = static_cast<std::size_t>(ch) + MINIMUM_REPEAT;
            repeating = true;
            value = readByte();
        }
    }

    static constexpr size_t MINIMUM_REPEAT = 3;
    size_t remainingValues = 0;
    char value = 0;
    const char* bufferStart;
    const char* bufferEnd;
    bool repeating = false;
};
} // namespace detail

/// Decode RLE bytes to a byte array
/// @param buffer The source data
/// @param out The target for output
/// @param numBytes The number of bytes to write, and the size of `out`
/// @param byteSize The number of bytes to consume from the source buffer
/// @throws std::runtime_error The provided buffer does not contain enough data
inline void decodeByte(BufferStream& buffer, std::uint8_t* out, count_t numBytes, count_t byteSize) {
    detail::ByteRleDecoder{buffer.getData(), buffer.getSize()}.next(out, numBytes);
    buffer.consume(byteSize);
}

/// Decode RLE bits to a byte array
/// @param buffer The source data
/// @param out The target for output
/// @param numBytes The number of bits to write, and the size of `out` multiplied by 8
/// @throws std::runtime_error The provided buffer does not contain enough data
/// @note Bit counts not divisible by 8 will be padded with zeros
inline void decodeBoolean(BufferStream& buffer, std::uint8_t* out, count_t numBits) {
    const auto numBytes = (numBits + 7) / 8;
    detail::ByteRleDecoder{buffer.getData(), buffer.getSize()}.next(out, numBytes);
}

/// Decode RLE bits to a vector
/// @param consume Whether to consume input the source buffer (true) or leave it unchanged (false)
void decodeBoolean(BufferStream&, std::vector<uint8_t>&, const metadata::stream::StreamMetadata&, bool consume);

/// Decode RLE bits to int/long values
/// @param buffer The source data
/// @param out The target for output
/// @param numRuns The number of RLE runs in the input
/// @param numValues The total number of values encoded, and the size of `out`
/// @throws std::runtime_error The provided buffer does not contain enough data
template <typename T>
    requires(std::is_integral_v<T> && !std::is_const_v<T> && 2 <= sizeof(T) && sizeof(T) <= 8)
void decodeInt(BufferStream& buffer, T* out, const count_t numRuns, const count_t numValues) {
    auto offset = 0;
    for (auto i = 0; i < numRuns; ++i) {
        auto runLength = buffer.read<std::uint32_t>();
        if (offset + runLength > numValues || !buffer.available(runLength * sizeof(T))) {
            throw std::runtime_error("Unexpected end of buffer");
        }
        std::memcpy(out + offset, buffer.getReadPosition(), runLength * sizeof(T));
        buffer.consume(runLength * sizeof(T));
        offset += runLength;
    }
}

#if 0
  public static int[] decodeUnsignedRLE(int[] data, int numRuns, int numTotalValues) {
    var values = new int[numTotalValues];
    var offset = 0;
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      for (var j = offset; j < offset + runLength; j++) {
        values[j] = value;
      }

      offset += runLength;
    }
    return values;
  }

  public static long[] decodeUnsignedRLE(long[] data, int numRuns, int numTotalValues) {
    var values = new long[numTotalValues];
    var offset = 0;
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      for (var j = offset; j < offset + runLength; j++) {
        values[j] = value;
      }

      offset += runLength;
    }
    return values;
  }
#endif

} // namespace mlt::util::decoding::rle
