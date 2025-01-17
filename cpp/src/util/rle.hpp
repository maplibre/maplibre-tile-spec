#pragma once

#include <common.hpp>

#include <cstdint>

namespace mlt::util::decoding::rle {

namespace detail {
// Borrowed from https://github.com/apache/orc, `/c++/src/ByteRLE.cc`, `ByteRleDecoderImpl::nextInternal`
// Apache License 2.0
struct ByteRleDecoder {
    ByteRleDecoder(const char* buffer, size_t length)
        : bufferStart(buffer),
          bufferEnd(buffer + length) {}

    static constexpr size_t MINIMUM_REPEAT = 3;
    size_t remainingValues = 0;
    char value = 0;
    const char* bufferStart;
    const char* bufferEnd;
    bool repeating = false;

    char readByte() {
        if (bufferStart == bufferEnd) {
            throw std::runtime_error("Unexpected end of buffer");
        }
        return *bufferStart++;
    }

    void readHeader() {
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
    void next(std::uint8_t* data, std::uint64_t numValues
#if SUPPORT_SKIP_NULL
    , const char* notNull
#endif
    ) {
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
};
} // namespace rle::detail

/// Decode RLE input to a byte array
/// @param buffer The source data
/// @param pos The starting position within the source data
/// @param out The target for output
/// @param numBytes The number of bytes to write, and the size of `out`
/// @param byteSize The number of bytes to consume from the source buffer
/// @throws std::runtime_error The provided buffer does not contain enough data
inline void decodeByte(DataView buffer, offset_t& pos, std::uint8_t* out, offset_t numBytes, offset_t byteSize) {
    detail::ByteRleDecoder{buffer.data(), buffer.size()}.next(out, numBytes);
    pos += byteSize;
}

/// Decode RLE input to a bitset
/// @param buffer The source data
/// @param pos The starting position within the source data
/// @param out The target for output
/// @param numBytes The number of bits to write, and the size of `out` multiplied by 8
/// @param byteSize The number of bytes to consume from the source buffer
/// @throws std::runtime_error The provided buffer does not contain enough data
/// @note Bit counts not divisible by 8 will be padded with zeros
inline void decodeBoolean(DataView buffer, offset_t& pos, std::uint8_t* out, offset_t numBits, offset_t byteSize) {
    const auto numBytes = (numBits + 7) / 8;
    detail::ByteRleDecoder{buffer.data(), buffer.size()}.next(out, numBytes);
    pos += byteSize;
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

}
