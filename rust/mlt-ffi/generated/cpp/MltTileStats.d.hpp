#ifndef MltTileStats_D_HPP
#define MltTileStats_D_HPP

#include "diplomat_runtime.hpp"
#include <cstdlib>
#include <functional>
#include <memory>
#include <optional>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

namespace diplomat::capi {
struct ConvertError;
}
class ConvertError;
struct MltLayerBytes;

namespace diplomat {
namespace capi {
struct MltTileStats;
} // namespace capi
} // namespace diplomat

/**
 * The bytes each layer of a tile spends, read without decoding the tile.
 */
class MltTileStats {
public:
    /**
     * Parse the layers of an MLT tile, leaving out any with a tag this build does not know.
     */
    inline static diplomat::result<std::unique_ptr<MltTileStats>, std::unique_ptr<ConvertError>> from_bytes(
        diplomat::span<const uint8_t> mlt);

    /**
     * Number of layers, in tile order.
     */
    inline size_t layer_count() const;

    /**
     * The name of layer `i`, or nothing past the last layer.
     */
    inline std::string layer_name(size_t i) const;
    template <typename W>
    inline void layer_name_write(size_t i, W& writeable_output) const;

    /**
     * The bytes of layer `i`, or `None` past the last layer.
     */
    inline std::optional<MltLayerBytes> layer_bytes(size_t i) const;

    inline const diplomat::capi::MltTileStats* AsFFI() const;
    inline diplomat::capi::MltTileStats* AsFFI();
    inline static const MltTileStats* FromFFI(const diplomat::capi::MltTileStats* ptr);
    inline static MltTileStats* FromFFI(diplomat::capi::MltTileStats* ptr);
    inline static void operator delete(void* ptr);

private:
    MltTileStats() = delete;
    MltTileStats(const MltTileStats&) = delete;
    MltTileStats(MltTileStats&&) noexcept = delete;
    MltTileStats operator=(const MltTileStats&) = delete;
    MltTileStats operator=(MltTileStats&&) noexcept = delete;
    static void operator delete[](void*, size_t) = delete;
};

#endif // MltTileStats_D_HPP
