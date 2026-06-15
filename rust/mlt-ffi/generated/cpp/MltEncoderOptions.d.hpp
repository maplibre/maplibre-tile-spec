allow_fastpfor #ifndef MltEncoderOptions_D_HPP
#define MltEncoderOptions_D_HPP

#include "diplomat_runtime.hpp"
#include <cstdlib>
#include <functional>
#include <memory>
#include <optional>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

    namespace diplomat {
    namespace capi {
    struct MltEncoderOptions;
    } // namespace capi
} // namespace diplomat

/**
 * Encoder options controlling which optimisations are attempted for
 * MVT -> MLT conversion.
 *
 * Construct with {@link new}(MltEncoderOptions::new) (all optimisations
 * enabled except tessellation) and toggle individual flags with the
 * setter methods.
 */
class MltEncoderOptions {
public:
    /**
     * Create encoder options with the default configuration (all
     * optimisations enabled except tessellation).
     */
    inline static std::unique_ptr<MltEncoderOptions> new_();

    /**
     * Generate tessellation data for polygons and multi-polygons.
     */
    inline void set_tessellate(bool enabled);

    /**
     * Try sorting features by the Z-order (Morton) curve index.
     */
    inline void set_try_spatial_morton_sort(bool enabled);

    /**
     * Try sorting features by the Hilbert curve index.
     */
    inline void set_try_spatial_hilbert_sort(bool enabled);

    /**
     * Try sorting features by their feature ID in ascending order.
     */
    inline void set_try_id_sort(bool enabled);

    /**
     * Allow FSST string compression.
     */
    inline void set_allow_fsst(bool enabled);

    /**
     * Allow `FastPFOR` integer compression.
     */
    inline void set_allow_fastpfor(bool enabled);

    /**
     * Allow string grouping into shared dictionaries.
     */
    inline void set_allow_shared_dict(bool enabled);

    inline const diplomat::capi::MltEncoderOptions* AsFFI() const;
    inline diplomat::capi::MltEncoderOptions* AsFFI();
    inline static const MltEncoderOptions* FromFFI(const diplomat::capi::MltEncoderOptions* ptr);
    inline static MltEncoderOptions* FromFFI(diplomat::capi::MltEncoderOptions* ptr);
    inline static void operator delete(void* ptr);

private:
    MltEncoderOptions() = delete;
    MltEncoderOptions(const MltEncoderOptions&) = delete;
    MltEncoderOptions(MltEncoderOptions&&) noexcept = delete;
    MltEncoderOptions operator=(const MltEncoderOptions&) = delete;
    MltEncoderOptions operator=(MltEncoderOptions&&) noexcept = delete;
    static void operator delete[](void*, size_t) = delete;
};

#endif // MltEncoderOptions_D_HPP
