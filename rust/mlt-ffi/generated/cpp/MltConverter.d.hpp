#ifndef MltConverter_D_HPP
#define MltConverter_D_HPP

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
struct MltBuffer;
}
class MltBuffer;
namespace diplomat::capi {
struct MltEncoderOptions;
}
class MltEncoderOptions;
class ConvertError;

namespace diplomat {
namespace capi {
struct MltConverter;
} // namespace capi
} // namespace diplomat

/**
 * Stateless FFI entry-points for MLT ↔ MVT conversion.
 */
class MltConverter {
public:
    /**
     * Decode MLT bytes into MVT bytes.
     */
    inline static diplomat::result<std::unique_ptr<MltBuffer>, ConvertError> mlt_to_mvt(
        diplomat::span<const uint8_t> mlt);

    /**
     * Encode MVT bytes into MLT bytes using the given encoder options.
     */
    inline static diplomat::result<std::unique_ptr<MltBuffer>, ConvertError> mvt_to_mlt(
        diplomat::span<const uint8_t> mvt, const MltEncoderOptions& options);

    inline const diplomat::capi::MltConverter* AsFFI() const;
    inline diplomat::capi::MltConverter* AsFFI();
    inline static const MltConverter* FromFFI(const diplomat::capi::MltConverter* ptr);
    inline static MltConverter* FromFFI(diplomat::capi::MltConverter* ptr);
    inline static void operator delete(void* ptr);

private:
    MltConverter() = delete;
    MltConverter(const MltConverter&) = delete;
    MltConverter(MltConverter&&) noexcept = delete;
    MltConverter operator=(const MltConverter&) = delete;
    MltConverter operator=(MltConverter&&) noexcept = delete;
    static void operator delete[](void*, size_t) = delete;
};

#endif // MltConverter_D_HPP
