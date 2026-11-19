#include <mlt/decode/fastpfor.hpp>

#include <fastpfor/bitpackinghelpers.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <span>
#include <stdexcept>
#include <string>
#include <vector>

namespace mlt::decoder::fastpfor {

namespace {

// Parameters of FastPFor<8> with its default page size
constexpr std::size_t packSize = 32; // values in a group of bit-packed values
constexpr std::size_t blockSize = 8 * packSize;
constexpr std::size_t pageSize = 65536;
constexpr std::size_t maxBits = 32;
constexpr std::size_t maxBlockExceptions = 255; // the count is stored in a byte

using Words = std::span<const std::uint32_t>;

[[noreturn]] void fail(const char* message) {
    throw std::runtime_error(std::string("Invalid FastPFOR stream: ") + message);
}

Words take(Words& in, const std::size_t count) {
    if (count > in.size()) {
        fail("unexpected end of data");
    }
    const auto head = in.first(count);
    in = in.subspan(count);
    return head;
}

std::uint32_t takeWord(Words& in) {
    return take(in, 1).front();
}

/// Reads the per-block bytes, which are not word-aligned
class ByteReader {
public:
    explicit ByteReader(std::span<const std::uint8_t> bytes_) noexcept
        : bytes(bytes_) {}

    std::uint8_t next() {
        if (bytes.empty()) {
            fail("unexpected end of block data");
        }
        const auto value = bytes.front();
        bytes = bytes.subspan(1);
        return value;
    }

private:
    std::span<const std::uint8_t> bytes;
};

/// Unpack `out.size()` values of `bits` bits each. Groups of 32 values take `bits` words, but the last group,
/// if incomplete, is not padded to a whole group.
/// @return Number of words consumed
std::size_t unpackTight(const Words in, const std::size_t bits, const std::span<std::uint32_t> out) {
    const auto words = (out.size() * bits + 31) / 32;
    if (words > in.size()) {
        fail("unexpected end of exception data");
    }

    std::size_t i = 0;
    std::size_t w = 0;
    for (; i + packSize <= out.size(); i += packSize, w += bits) {
        FastPForLib::fastunpack(&in[w], &out[i], static_cast<std::uint32_t>(bits));
    }
    if (i < out.size()) {
        // The unpacker reads and writes whole groups, so complete the last one with the zeros it was padded with
        std::array<std::uint32_t, maxBits> packed{};
        std::array<std::uint32_t, packSize> unpacked{};
        std::copy(in.begin() + static_cast<std::ptrdiff_t>(w),
                  in.begin() + static_cast<std::ptrdiff_t>(words),
                  packed.begin());
        FastPForLib::fastunpack(packed.data(), unpacked.data(), static_cast<std::uint32_t>(bits));
        std::copy_n(unpacked.begin(), out.size() - i, &out[i]);
    }
    return words;
}

/// Decode a page of whole blocks.
/// Layout: [offset of the metadata | bit-packed blocks | metadata], where the metadata is
///   [byte count | block bytes, padded to words | exception bitmap | exception lists]
/// @param in Input, starting at the page
/// @param out Output for the page, a multiple of the block size
/// @param exceptions Scratch space for exception values
/// @return The input following the page
Words decodePage(Words in, const std::span<std::uint32_t> out, std::vector<std::uint32_t>& exceptions) {
    const std::size_t metadataOffset = takeWord(in);
    if (metadataOffset == 0) {
        fail("invalid metadata offset");
    }
    auto packed = take(in, metadataOffset - 1);

    const std::size_t byteCount = takeWord(in);
    const auto byteWords = take(in, byteCount / sizeof(std::uint32_t) + (byteCount % sizeof(std::uint32_t) != 0));
    ByteReader bytes({reinterpret_cast<const std::uint8_t*>(byteWords.data()), byteCount});

    // Values that did not fit in the bit width of their block are stored as exceptions: the bits above that width,
    // in a list per number of bits (but not when it's a single bit, which is implied).
    const auto bitmap = takeWord(in);
    const std::size_t maxExceptions = (out.size() / blockSize) * maxBlockExceptions;
    exceptions.clear();
    std::array<std::size_t, maxBits + 1> nextException{};
    std::array<std::size_t, maxBits + 1> endException{};
    for (std::size_t bits = 2; bits <= maxBits; ++bits) {
        if ((bitmap & (std::uint32_t{1} << (bits - 1))) == 0) {
            continue;
        }
        const std::size_t count = takeWord(in);
        if (count > maxExceptions - exceptions.size()) {
            fail("too many exceptions");
        }
        const auto begin = exceptions.size();
        exceptions.resize(begin + count);
        in = in.subspan(unpackTight(in, bits, {exceptions.data() + begin, count}));
        nextException[bits] = begin;
        endException[bits] = exceptions.size();
    }

    for (std::size_t blockStart = 0; blockStart < out.size(); blockStart += blockSize) {
        const auto block = out.subspan(blockStart, blockSize);

        const std::size_t bits = bytes.next();
        const std::size_t exceptionCount = bytes.next();
        if (bits > maxBits) {
            fail("invalid bit width");
        }
        const auto blockWords = (blockSize / packSize) * bits;
        if (blockWords > packed.size()) {
            fail("block extends beyond packed data");
        }
        for (std::size_t i = 0; i < blockSize; i += packSize, packed = packed.subspan(bits)) {
            FastPForLib::fastunpack(packed.data(), &block[i], static_cast<std::uint32_t>(bits));
        }

        if (exceptionCount > 0) {
            const std::size_t maxBitsInBlock = bytes.next();
            if (maxBitsInBlock <= bits || maxBitsInBlock > maxBits) {
                fail("invalid exception bit width");
            }
            const auto exceptionBits = maxBitsInBlock - bits;
            for (std::size_t i = 0; i < exceptionCount; ++i) {
                const std::size_t position = bytes.next(); // < blockSize, as it comes from a byte
                std::uint32_t high = 1;
                if (exceptionBits > 1) {
                    if (nextException[exceptionBits] == endException[exceptionBits]) {
                        fail("missing exception value");
                    }
                    high = exceptions[nextException[exceptionBits]++];
                }
                block[position] |= high << bits;
            }
        }
    }
    if (!packed.empty()) {
        fail("unused packed data");
    }
    return in;
}

/// Decode `out.size()` values from the variable-byte stream, 7 bits per byte, least significant first.
/// The last byte of each value has its top bit set.
void decodeVariableByte(const Words in, const std::span<std::uint32_t> out) {
    const std::span<const std::uint8_t> bytes{reinterpret_cast<const std::uint8_t*>(in.data()), in.size_bytes()};
    std::size_t index = 0;
    for (auto& value : out) {
        std::uint32_t result = 0;
        for (unsigned shift = 0;; shift += 7) {
            if (index == bytes.size()) {
                fail("fewer values than expected");
            }
            const auto byte = bytes[index++];
            const std::uint32_t payload = byte & 0x7f;
            if (shift == 28 && payload > 0x0f) {
                fail("variable-byte value exceeds 32 bits");
            }
            result |= payload << shift;
            if ((byte & 0x80) != 0) {
                break;
            }
            if (shift == 28) {
                fail("variable-byte value exceeds 32 bits");
            }
        }
        value = result;
    }

    // What remains is padding, which has no terminated values
    if (std::any_of(
            bytes.begin() + static_cast<std::ptrdiff_t>(index), bytes.end(), [](auto b) { return (b & 0x80) != 0; })) {
        fail("more values than expected");
    }
}

} // namespace

void decode(Words in, const std::span<std::uint32_t> out) {
    if (out.empty()) {
        return;
    }

    // Whole blocks are compressed with FastPFor, the remainder with VariableByte
    const std::size_t blockedCount = takeWord(in);
    if (blockedCount > out.size() || blockedCount % blockSize != 0) {
        fail("invalid value count");
    }

    std::vector<std::uint32_t> exceptions;
    for (std::size_t start = 0; start < blockedCount; start += pageSize) {
        in = decodePage(in, out.subspan(start, std::min(pageSize, blockedCount - start)), exceptions);
    }

    decodeVariableByte(in, out.subspan(blockedCount));
}

} // namespace mlt::decoder::fastpfor
