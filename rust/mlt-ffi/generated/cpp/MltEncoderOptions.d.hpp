#ifndef MltEncoderOptions_D_HPP
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

class MltWireVersion;

namespace diplomat {
namespace capi {
struct MltEncoderOptions;
} // namespace capi
} // namespace diplomat

/**
 * Encoder options controlling which optimisations are attempted for
 * MVT -> MLT conversion.
 *
 * Construct with {@link new}(MltEncoderOptions::new) (FSST, `FastPFOR` and shared
 * dictionaries enabled, no sorting and no tessellation) and toggle individual flags with the
 * setter methods.
 */
class MltEncoderOptions {
public:
    /**
     * Create encoder options with the default configuration.
     */
    inline static std::unique_ptr<MltEncoderOptions> new_();

    /**
     * Generate tessellation data for polygons and multi-polygons.
     */
    inline void set_tessellate(bool enabled);

    /**
     * Try sorting features by the Z-order (Morton) curve index.
     */
    inline void set_attempt_spatial_morton_sort(bool enabled);

    /**
     * Try sorting features by the Hilbert curve index.
     */
    inline void set_attempt_spatial_hilbert_sort(bool enabled);

    /**
     * Try sorting features by their feature ID in ascending order.
     */
    inline void set_attempt_id_sort(bool enabled);

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

    /**
     * Select the wire format to encode to.
     * Every setter marked v2 only has no effect on a v1 layer.
     */
    inline void set_wire_version(MltWireVersion version);

    /**
     * v2 only: let a tessellated all-polygon layer store its triangles without the outlines.
     * Each polygon then decodes as the triangles it was cut into.
     * Requires tessellation.
     */
    inline void set_allow_triangles_only(bool enabled);

    /**
     * v2 only: allow integer and vertex streams to store the deltas of their deltas.
     */
    inline void set_allow_delta2(bool enabled);

    /**
     * v2 only: allow float columns to store one code per value into a dictionary.
     */
    inline void set_allow_float_dict(bool enabled);

    /**
     * v2 only: allow float columns to store each value as a decimal-scaled integer (ALP).
     */
    inline void set_allow_float_alp(bool enabled);

    /**
     * v2 only: allow dictionary code streams to be bit-packed.
     */
    inline void set_allow_packed_dict_codes(bool enabled);

    /**
     * v2 only: allow plain vertex streams to be rANS-coded.
     */
    inline void set_allow_rans_vertices(bool enabled);

    /**
     * v2 only: let a nested struct or map code its row shapes instead of per-field presence.
     */
    inline void set_allow_row_shapes(bool enabled);

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
