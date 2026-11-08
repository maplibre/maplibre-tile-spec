// Fuzz the FastPFOR stream decoder.
//
// Input: a little-endian 32-bit count of values to decode, followed by the words of the stream.

#include <mlt/decode/fastpfor.hpp>

#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <stdexcept>
#include <vector>

namespace {
// Decode into a buffer of the given initial contents, returning whether it succeeded
bool decode(const std::vector<std::uint32_t>& words, std::vector<std::uint32_t>& out) {
    try {
        mlt::decoder::fastpfor::decode(words, out);
        return true;
    } catch (const std::runtime_error&) {
        return false;
    }
}
} // namespace

extern "C" int LLVMFuzzerTestOneInput(const std::uint8_t* data, std::size_t size) {
    constexpr std::size_t maxValues = 1 << 20;
    if (size < sizeof(std::uint32_t)) {
        return 0;
    }
    std::uint32_t count = 0;
    std::memcpy(&count, data, sizeof(count));
    count %= maxValues + 1;

    std::vector<std::uint32_t> words((size - sizeof(count)) / sizeof(std::uint32_t));
    if (!words.empty()) {
        std::memcpy(words.data(), data + sizeof(count), words.size() * sizeof(std::uint32_t));
    }

    // Whatever the output buffer held beforehand must not matter: a stream that decodes has to set every value
    std::vector<std::uint32_t> a(count, 0xaaaaaaaau);
    std::vector<std::uint32_t> b(count, 0x55555555u);
    const bool okA = decode(words, a);
    const bool okB = decode(words, b);
    if (okA != okB || (okA && a != b)) {
        std::abort();
    }
    return 0;
}
