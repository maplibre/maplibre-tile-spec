#ifndef MltEncoderOptions_HPP
#define MltEncoderOptions_HPP

#include "MltEncoderOptions.d.hpp"

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

diplomat::capi::MltEncoderOptions* MltEncoderOptions_new(void);

void MltEncoderOptions_set_tessellate(diplomat::capi::MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_attempt_spatial_morton_sort(diplomat::capi::MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_attempt_spatial_hilbert_sort(diplomat::capi::MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_attempt_id_sort(diplomat::capi::MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_fsst(diplomat::capi::MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_fastpfor(diplomat::capi::MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_shared_dict(diplomat::capi::MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_destroy(MltEncoderOptions* self);

} // extern "C"
} // namespace capi
} // namespace diplomat

inline std::unique_ptr<MltEncoderOptions> MltEncoderOptions::new_() {
    auto result = diplomat::capi::MltEncoderOptions_new();
    return std::unique_ptr<MltEncoderOptions>(MltEncoderOptions::FromFFI(result));
}

inline void MltEncoderOptions::set_tessellate(bool enabled) {
    diplomat::capi::MltEncoderOptions_set_tessellate(this->AsFFI(), enabled);
}

inline void MltEncoderOptions::set_attempt_spatial_morton_sort(bool enabled) {
    diplomat::capi::MltEncoderOptions_set_attempt_spatial_morton_sort(this->AsFFI(), enabled);
}

inline void MltEncoderOptions::set_attempt_spatial_hilbert_sort(bool enabled) {
    diplomat::capi::MltEncoderOptions_set_attempt_spatial_hilbert_sort(this->AsFFI(), enabled);
}

inline void MltEncoderOptions::set_attempt_id_sort(bool enabled) {
    diplomat::capi::MltEncoderOptions_set_attempt_id_sort(this->AsFFI(), enabled);
}

inline void MltEncoderOptions::set_allow_fsst(bool enabled) {
    diplomat::capi::MltEncoderOptions_set_allow_fsst(this->AsFFI(), enabled);
}

inline void MltEncoderOptions::set_allow_fastpfor(bool enabled) {
    diplomat::capi::MltEncoderOptions_set_allow_fastpfor(this->AsFFI(), enabled);
}

inline void MltEncoderOptions::set_allow_shared_dict(bool enabled) {
    diplomat::capi::MltEncoderOptions_set_allow_shared_dict(this->AsFFI(), enabled);
}

inline const diplomat::capi::MltEncoderOptions* MltEncoderOptions::AsFFI() const {
    return reinterpret_cast<const diplomat::capi::MltEncoderOptions*>(this);
}

inline diplomat::capi::MltEncoderOptions* MltEncoderOptions::AsFFI() {
    return reinterpret_cast<diplomat::capi::MltEncoderOptions*>(this);
}

inline const MltEncoderOptions* MltEncoderOptions::FromFFI(const diplomat::capi::MltEncoderOptions* ptr) {
    return reinterpret_cast<const MltEncoderOptions*>(ptr);
}

inline MltEncoderOptions* MltEncoderOptions::FromFFI(diplomat::capi::MltEncoderOptions* ptr) {
    return reinterpret_cast<MltEncoderOptions*>(ptr);
}

inline void MltEncoderOptions::operator delete(void* ptr) {
    diplomat::capi::MltEncoderOptions_destroy(reinterpret_cast<diplomat::capi::MltEncoderOptions*>(ptr));
}

#endif // MltEncoderOptions_HPP
