#pragma once

#include <cstddef>
#include <limits>
#include <stdexcept>

namespace mlt::util {

/// Limits the memory used to decode a tile.
///
/// The sizes of many decoded structures come from the tile and are not limited by its size: a few bytes of
/// run-length encoding can describe billions of values. Decoders charge the budget before allocating for those.
class MemoryBudget {
public:
    /// The default limit, which is the same as for the Rust decoder
    static constexpr std::size_t defaultLimit = 64 * 1024 * 1024;

    explicit MemoryBudget(std::size_t limit_ = defaultLimit) noexcept
        : limit(limit_) {}

    std::size_t getLimit() const noexcept { return limit; }
    std::size_t getUsed() const noexcept { return used; }

    void setLimit(std::size_t limit_) noexcept { limit = limit_; }

    /// Forget previous usage, e.g. when starting a new tile
    void reset() noexcept { used = 0; }

    /// Charge for an allocation which is about to be made
    /// @throws std::runtime_error if the allocation would exceed the limit
    void consume(std::size_t bytes) {
        if (used > limit || bytes > limit - used) {
            throw std::runtime_error("Memory limit exceeded");
        }
        used += bytes;
    }

    /// Charge for an allocation of `count` items
    template <typename T>
    void consumeItems(std::size_t count) {
        if (count > std::numeric_limits<std::size_t>::max() / sizeof(T)) {
            throw std::runtime_error("Memory limit exceeded");
        }
        consume(count * sizeof(T));
    }

private:
    std::size_t limit;
    std::size_t used = 0;
};

} // namespace mlt::util
