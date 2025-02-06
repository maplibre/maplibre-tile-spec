#pragma once

#include <bit>
#include <cassert>
#include <numeric>
#include <optional>
#include <vector>

namespace mlt {

// `std::vector<bool>` is not great, just use a vector of bytes
using PackedBitset = std::vector<std::uint8_t>;

/// Test a specific bit
static inline bool testBit(const PackedBitset& bitset, std::size_t i) noexcept {
    return ((i / 8) < bitset.size()) && (bitset[i / 8] & (1 << (i % 8)));
}

/// Get the total number of bits set
static inline std::size_t countSetBits(const PackedBitset& bitset) noexcept(false) {
    return std::accumulate(
        bitset.begin(), bitset.end(), 0, [](const auto total, const auto byte) { return total + std::popcount(byte); });
}

/// Return the index of the next set bit within the bitstream
/// @param bits The bitset
/// @param afterIndex The bit index to start with
/// @return The index of the next set bit (including the starting index)
static inline std::optional<std::size_t> nextSetBit(const PackedBitset& bits,
                                                    const std::size_t afterIndex = 0) noexcept {
    if (std::size_t byteIndex = (afterIndex / 8); byteIndex < bits.size()) {
        auto byte = bits[byteIndex];

        // If we're mid-byte, shift it down so the next bit is in the 1 position
        std::size_t result = afterIndex;
        if (const auto partialBits = result & 7; partialBits) {
            byte >>= partialBits;
            if (!byte) {
                // skip to the next byte
                if (++byteIndex == bits.size()) {
                    return {};
                }
                result += (8 - partialBits);
                byte = bits[byteIndex];
            }
        }

        for (; byteIndex < bits.size(); result += 8, byte = bits[byteIndex]) {
            // If this byte is non-zero, the next bit is within it
            if (byte) {
                const auto ffs = std::countr_zero(byte);
                assert(ffs < 8);
                return result + ffs;
            }
            // Continue to the next byte
            ++byteIndex;
        }
    }
    return {};
}

} // namespace mlt
