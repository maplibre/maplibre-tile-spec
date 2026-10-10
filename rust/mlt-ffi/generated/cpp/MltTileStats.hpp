#ifndef MltTileStats_HPP
#define MltTileStats_HPP

#include "MltTileStats.d.hpp"

#include "ConvertError.hpp"
#include "MltLayerBytes.hpp"
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

typedef struct MltTileStats_from_bytes_result {
    union {
        diplomat::capi::MltTileStats* ok;
        diplomat::capi::ConvertError* err;
    };
    bool is_ok;
} MltTileStats_from_bytes_result;
MltTileStats_from_bytes_result MltTileStats_from_bytes(diplomat::capi::DiplomatU8View mlt);

size_t MltTileStats_layer_count(const diplomat::capi::MltTileStats* self);

void MltTileStats_layer_name(const diplomat::capi::MltTileStats* self, size_t i, diplomat::capi::DiplomatWrite* write);

typedef struct MltTileStats_layer_bytes_result {
    union {
        diplomat::capi::MltLayerBytes ok;
    };
    bool is_ok;
} MltTileStats_layer_bytes_result;
MltTileStats_layer_bytes_result MltTileStats_layer_bytes(const diplomat::capi::MltTileStats* self, size_t i);

void MltTileStats_destroy(MltTileStats* self);

} // extern "C"
} // namespace capi
} // namespace diplomat

inline diplomat::result<std::unique_ptr<MltTileStats>, std::unique_ptr<ConvertError>> MltTileStats::from_bytes(
    diplomat::span<const uint8_t> mlt) {
    auto result = diplomat::capi::MltTileStats_from_bytes({mlt.data(), mlt.size()});
    return result.is_ok ? diplomat::result<std::unique_ptr<MltTileStats>, std::unique_ptr<ConvertError>>(
                              diplomat::Ok<std::unique_ptr<MltTileStats>>(
                                  std::unique_ptr<MltTileStats>(MltTileStats::FromFFI(result.ok))))
                        : diplomat::result<std::unique_ptr<MltTileStats>, std::unique_ptr<ConvertError>>(
                              diplomat::Err<std::unique_ptr<ConvertError>>(
                                  std::unique_ptr<ConvertError>(ConvertError::FromFFI(result.err))));
}

inline size_t MltTileStats::layer_count() const {
    auto result = diplomat::capi::MltTileStats_layer_count(this->AsFFI());
    return result;
}

inline std::string MltTileStats::layer_name(size_t i) const {
    std::string output;
    diplomat::capi::DiplomatWrite write = diplomat::WriteFromString(output);
    diplomat::capi::MltTileStats_layer_name(this->AsFFI(), i, &write);
    return output;
}
template <typename W>
inline void MltTileStats::layer_name_write(size_t i, W& writeable) const {
    diplomat::capi::DiplomatWrite write = diplomat::WriteTrait<W>::Construct(writeable);
    diplomat::capi::MltTileStats_layer_name(this->AsFFI(), i, &write);
}

inline std::optional<MltLayerBytes> MltTileStats::layer_bytes(size_t i) const {
    auto result = diplomat::capi::MltTileStats_layer_bytes(this->AsFFI(), i);
    return result.is_ok ? std::optional<MltLayerBytes>(MltLayerBytes::FromFFI(result.ok)) : std::nullopt;
}

inline const diplomat::capi::MltTileStats* MltTileStats::AsFFI() const {
    return reinterpret_cast<const diplomat::capi::MltTileStats*>(this);
}

inline diplomat::capi::MltTileStats* MltTileStats::AsFFI() {
    return reinterpret_cast<diplomat::capi::MltTileStats*>(this);
}

inline const MltTileStats* MltTileStats::FromFFI(const diplomat::capi::MltTileStats* ptr) {
    return reinterpret_cast<const MltTileStats*>(ptr);
}

inline MltTileStats* MltTileStats::FromFFI(diplomat::capi::MltTileStats* ptr) {
    return reinterpret_cast<MltTileStats*>(ptr);
}

inline void MltTileStats::operator delete(void* ptr) {
    diplomat::capi::MltTileStats_destroy(reinterpret_cast<diplomat::capi::MltTileStats*>(ptr));
}

#endif // MltTileStats_HPP
