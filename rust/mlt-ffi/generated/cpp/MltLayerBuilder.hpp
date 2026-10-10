#ifndef MltLayerBuilder_HPP
#define MltLayerBuilder_HPP

#include "MltLayerBuilder.d.hpp"

#include "ConvertError.hpp"
#include "MltBuffer.hpp"
#include "MltEncoderOptions.hpp"
#include "MltGeometryType.hpp"
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
    extern "C" {

    typedef struct MltLayerBuilder_new_result {
        union {
            diplomat::capi::MltLayerBuilder* ok;
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_new_result;
    MltLayerBuilder_new_result MltLayerBuilder_new(diplomat::capi::DiplomatStringView name, uint32_t extent);

    typedef struct MltLayerBuilder_reset_result {
        union {
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_reset_result;
    MltLayerBuilder_reset_result MltLayerBuilder_reset(diplomat::capi::MltLayerBuilder* self, diplomat::capi::DiplomatStringView name, uint32_t extent);

    uint32_t MltLayerBuilder_add_property(diplomat::capi::MltLayerBuilder* self, diplomat::capi::DiplomatStringView name);

    void MltLayerBuilder_begin_feature(diplomat::capi::MltLayerBuilder* self, diplomat::capi::MltGeometryType geometry, diplomat::capi::OptionU64 id);

    typedef struct MltLayerBuilder_add_points_result {
        union {
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_add_points_result;
    MltLayerBuilder_add_points_result MltLayerBuilder_add_points(diplomat::capi::MltLayerBuilder* self, diplomat::capi::DiplomatI32View xy);

    typedef struct MltLayerBuilder_add_line_result {
        union {
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_add_line_result;
    MltLayerBuilder_add_line_result MltLayerBuilder_add_line(diplomat::capi::MltLayerBuilder* self, diplomat::capi::DiplomatI32View xy);

    typedef struct MltLayerBuilder_add_exterior_ring_result {
        union {
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_add_exterior_ring_result;
    MltLayerBuilder_add_exterior_ring_result MltLayerBuilder_add_exterior_ring(diplomat::capi::MltLayerBuilder* self, diplomat::capi::DiplomatI32View xy);

    typedef struct MltLayerBuilder_add_hole_result {
        union {
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_add_hole_result;
    MltLayerBuilder_add_hole_result MltLayerBuilder_add_hole(diplomat::capi::MltLayerBuilder* self, diplomat::capi::DiplomatI32View xy);

    typedef struct MltLayerBuilder_set_bool_result {
        union {
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_set_bool_result;
    MltLayerBuilder_set_bool_result MltLayerBuilder_set_bool(diplomat::capi::MltLayerBuilder* self, uint32_t key, bool value);

    typedef struct MltLayerBuilder_set_i64_result {
        union {
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_set_i64_result;
    MltLayerBuilder_set_i64_result MltLayerBuilder_set_i64(diplomat::capi::MltLayerBuilder* self, uint32_t key, int64_t value);

    typedef struct MltLayerBuilder_set_f32_result {
        union {
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_set_f32_result;
    MltLayerBuilder_set_f32_result MltLayerBuilder_set_f32(diplomat::capi::MltLayerBuilder* self, uint32_t key, float value);

    typedef struct MltLayerBuilder_set_f64_result {
        union {
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_set_f64_result;
    MltLayerBuilder_set_f64_result MltLayerBuilder_set_f64(diplomat::capi::MltLayerBuilder* self, uint32_t key, double value);

    typedef struct MltLayerBuilder_set_str_result {
        union {
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_set_str_result;
    MltLayerBuilder_set_str_result MltLayerBuilder_set_str(diplomat::capi::MltLayerBuilder* self, uint32_t key, diplomat::capi::DiplomatStringView value);

    typedef struct MltLayerBuilder_encode_into_result {
        union {
            diplomat::capi::ConvertError* err;
        };
        bool is_ok;
    } MltLayerBuilder_encode_into_result;
    MltLayerBuilder_encode_into_result MltLayerBuilder_encode_into(const diplomat::capi::MltLayerBuilder* self, const diplomat::capi::MltEncoderOptions* options, diplomat::capi::MltBuffer* out);

    void MltLayerBuilder_destroy(MltLayerBuilder* self);

    } // extern "C"
} // namespace capi
} // namespace

inline diplomat::result<diplomat::result<std::unique_ptr<MltLayerBuilder>, std::unique_ptr<ConvertError>>, diplomat::Utf8Error> MltLayerBuilder::new_(std::string_view name, uint32_t extent)
{
    if (!diplomat::capi::diplomat_is_str(name.data(), name.size())) {
        return diplomat::Err<diplomat::Utf8Error>();
    }
    auto result = diplomat::capi::MltLayerBuilder_new({ name.data(), name.size() },
        extent);
    return diplomat::Ok<diplomat::result<std::unique_ptr<MltLayerBuilder>, std::unique_ptr<ConvertError>>>(result.is_ok ? diplomat::result<std::unique_ptr<MltLayerBuilder>, std::unique_ptr<ConvertError>>(diplomat::Ok<std::unique_ptr<MltLayerBuilder>>(std::unique_ptr<MltLayerBuilder>(MltLayerBuilder::FromFFI(result.ok)))) : diplomat::result<std::unique_ptr<MltLayerBuilder>, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err)))));
}

inline diplomat::result<diplomat::result<std::monostate, std::unique_ptr<ConvertError>>, diplomat::Utf8Error> MltLayerBuilder::reset(std::string_view name, uint32_t extent)
{
    if (!diplomat::capi::diplomat_is_str(name.data(), name.size())) {
        return diplomat::Err<diplomat::Utf8Error>();
    }
    auto result = diplomat::capi::MltLayerBuilder_reset(this->AsFFI(),
        { name.data(), name.size() },
        extent);
    return diplomat::Ok<diplomat::result<std::monostate, std::unique_ptr<ConvertError>>>(result.is_ok ? diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Ok<std::monostate>()) : diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err)))));
}

inline diplomat::result<uint32_t, diplomat::Utf8Error> MltLayerBuilder::add_property(std::string_view name)
{
    if (!diplomat::capi::diplomat_is_str(name.data(), name.size())) {
        return diplomat::Err<diplomat::Utf8Error>();
    }
    auto result = diplomat::capi::MltLayerBuilder_add_property(this->AsFFI(),
        { name.data(), name.size() });
    return diplomat::Ok<uint32_t>(result);
}

inline void MltLayerBuilder::begin_feature(MltGeometryType geometry, std::optional<uint64_t> id)
{
    diplomat::capi::MltLayerBuilder_begin_feature(this->AsFFI(),
        geometry.AsFFI(),
        id.has_value() ? (diplomat::capi::OptionU64 { { id.value() }, true }) : (diplomat::capi::OptionU64 { {}, false }));
}

inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> MltLayerBuilder::add_points(diplomat::span<const int32_t> xy)
{
    auto result = diplomat::capi::MltLayerBuilder_add_points(this->AsFFI(),
        { xy.data(), xy.size() });
    return result.is_ok ? diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Ok<std::monostate>()) : diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err))));
}

inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> MltLayerBuilder::add_line(diplomat::span<const int32_t> xy)
{
    auto result = diplomat::capi::MltLayerBuilder_add_line(this->AsFFI(),
        { xy.data(), xy.size() });
    return result.is_ok ? diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Ok<std::monostate>()) : diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err))));
}

inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> MltLayerBuilder::add_exterior_ring(diplomat::span<const int32_t> xy)
{
    auto result = diplomat::capi::MltLayerBuilder_add_exterior_ring(this->AsFFI(),
        { xy.data(), xy.size() });
    return result.is_ok ? diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Ok<std::monostate>()) : diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err))));
}

inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> MltLayerBuilder::add_hole(diplomat::span<const int32_t> xy)
{
    auto result = diplomat::capi::MltLayerBuilder_add_hole(this->AsFFI(),
        { xy.data(), xy.size() });
    return result.is_ok ? diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Ok<std::monostate>()) : diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err))));
}

inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> MltLayerBuilder::set_bool(uint32_t key, bool value)
{
    auto result = diplomat::capi::MltLayerBuilder_set_bool(this->AsFFI(),
        key,
        value);
    return result.is_ok ? diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Ok<std::monostate>()) : diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err))));
}

inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> MltLayerBuilder::set_i64(uint32_t key, int64_t value)
{
    auto result = diplomat::capi::MltLayerBuilder_set_i64(this->AsFFI(),
        key,
        value);
    return result.is_ok ? diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Ok<std::monostate>()) : diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err))));
}

inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> MltLayerBuilder::set_f32(uint32_t key, float value)
{
    auto result = diplomat::capi::MltLayerBuilder_set_f32(this->AsFFI(),
        key,
        value);
    return result.is_ok ? diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Ok<std::monostate>()) : diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err))));
}

inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> MltLayerBuilder::set_f64(uint32_t key, double value)
{
    auto result = diplomat::capi::MltLayerBuilder_set_f64(this->AsFFI(),
        key,
        value);
    return result.is_ok ? diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Ok<std::monostate>()) : diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err))));
}

inline diplomat::result<diplomat::result<std::monostate, std::unique_ptr<ConvertError>>, diplomat::Utf8Error> MltLayerBuilder::set_str(uint32_t key, std::string_view value)
{
    if (!diplomat::capi::diplomat_is_str(value.data(), value.size())) {
        return diplomat::Err<diplomat::Utf8Error>();
    }
    auto result = diplomat::capi::MltLayerBuilder_set_str(this->AsFFI(),
        key,
        { value.data(), value.size() });
    return diplomat::Ok<diplomat::result<std::monostate, std::unique_ptr<ConvertError>>>(result.is_ok ? diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Ok<std::monostate>()) : diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err)))));
}

inline diplomat::result<std::monostate, std::unique_ptr<ConvertError>> MltLayerBuilder::encode_into(const MltEncoderOptions& options, MltBuffer& out) const
{
    auto result = diplomat::capi::MltLayerBuilder_encode_into(this->AsFFI(),
        options.AsFFI(),
        out.AsFFI());
    return result.is_ok ? diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Ok<std::monostate>()) : diplomat::result<std::monostate, std::unique_ptr<ConvertError>>(diplomat::Err<std::unique_ptr<ConvertError>>(std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err))));
}

inline const diplomat::capi::MltLayerBuilder* MltLayerBuilder::AsFFI() const
{
    return reinterpret_cast<const diplomat::capi::MltLayerBuilder*>(this);
}

inline diplomat::capi::MltLayerBuilder* MltLayerBuilder::AsFFI()
{
    return reinterpret_cast<diplomat::capi::MltLayerBuilder*>(this);
}

inline const MltLayerBuilder* MltLayerBuilder::FromFFI(const diplomat::capi::MltLayerBuilder* ptr)
{
    return reinterpret_cast<const MltLayerBuilder*>(ptr);
}

inline MltLayerBuilder* MltLayerBuilder::FromFFI(diplomat::capi::MltLayerBuilder* ptr)
{
    return reinterpret_cast<MltLayerBuilder*>(ptr);
}

inline void MltLayerBuilder::operator delete(void* ptr)
{
    diplomat::capi::MltLayerBuilder_destroy(reinterpret_cast<diplomat::capi::MltLayerBuilder*>(ptr));
}

#endif // MltLayerBuilder_HPP
