#ifndef MltLayerBuilder_D_HPP
#define MltLayerBuilder_D_HPP

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
namespace diplomat::capi {
struct MltBuffer;
}
class MltBuffer;
namespace diplomat::capi {
struct MltEncoderOptions;
}
class MltEncoderOptions;
class MltMvtGeometryType;

namespace diplomat {
namespace capi {
struct MltLayerBuilder;
} // namespace capi
} // namespace diplomat

/**
 * A layer written one feature at a time, then encoded without going through MVT.
 */
class MltLayerBuilder {
public:
    /**
     * Start a layer.
     */
    inline static diplomat::result<diplomat::result<std::unique_ptr<MltLayerBuilder>, std::unique_ptr<ConvertError>>,
                                   diplomat::Utf8Error>
    new_(std::string_view name, uint32_t extent);

    /**
     * Start another layer, keeping the allocations of the last one.
     */
    inline diplomat::result<diplomat::result<std::monostate, std::unique_ptr<ConvertError>>, diplomat::Utf8Error> reset(
        std::string_view name, uint32_t extent);

    /**
     * Declare a property column, returning the key to set it with.
     * Declaring a name again returns its existing key.
     */
    inline diplomat::result<uint32_t, diplomat::Utf8Error> add_property(std::string_view name);

    /**
     * Start a feature with its geometry given as MVT commands, ending the previous one.
     * A ring with positive area starts a polygon and any other ring is a hole of the last one.
     */
    inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> begin_mvt_feature(
        MltMvtGeometryType geometry, diplomat::span<const uint32_t> commands, std::optional<uint64_t> id);

    /**
     * Set a boolean property of the current feature.
     */
    inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> set_bool(uint32_t key, bool value);

    /**
     * Set an integer property of the current feature.
     */
    inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> set_i64(uint32_t key, int64_t value);

    /**
     * Set a 32-bit float property of the current feature.
     */
    inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> set_f32(uint32_t key, float value);

    /**
     * Set a 64-bit float property of the current feature.
     */
    inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> set_f64(uint32_t key, double value);

    /**
     * Set a string property of the current feature.
     */
    inline diplomat::result<diplomat::result<std::monostate, std::unique_ptr<ConvertError>>, diplomat::Utf8Error>
    set_str(uint32_t key, std::string_view value);

    /**
     * Encode the layer and append it to `out`.
     */
    inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> encode_into(const MltEncoderOptions& options,
                                                                                       MltBuffer& out) const;

    inline const diplomat::capi::MltLayerBuilder* AsFFI() const;
    inline diplomat::capi::MltLayerBuilder* AsFFI();
    inline static const MltLayerBuilder* FromFFI(const diplomat::capi::MltLayerBuilder* ptr);
    inline static MltLayerBuilder* FromFFI(diplomat::capi::MltLayerBuilder* ptr);
    inline static void operator delete(void* ptr);

private:
    MltLayerBuilder() = delete;
    MltLayerBuilder(const MltLayerBuilder&) = delete;
    MltLayerBuilder(MltLayerBuilder&&) noexcept = delete;
    MltLayerBuilder operator=(const MltLayerBuilder&) = delete;
    MltLayerBuilder operator=(MltLayerBuilder&&) noexcept = delete;
    static void operator delete[](void*, size_t) = delete;
};

#endif // MltLayerBuilder_D_HPP
