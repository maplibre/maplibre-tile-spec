#ifndef MltBuffer_HPP
#define MltBuffer_HPP

#include "MltBuffer.d.hpp"

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <memory>
#include <functional>
#include <optional>
#include <cstdlib>
#include "diplomat_runtime.hpp"

namespace diplomat {
namespace capi {
extern "C" {

diplomat::capi::DiplomatU8View MltBuffer_as_bytes(const diplomat::capi::MltBuffer* self);

size_t MltBuffer_len(const diplomat::capi::MltBuffer* self);

void MltBuffer_destroy(MltBuffer* self);

} // extern "C"
} // namespace capi
} // namespace diplomat

inline diplomat::span<const uint8_t> MltBuffer::as_bytes() const {
    auto result = diplomat::capi::MltBuffer_as_bytes(this->AsFFI());
    return diplomat::span<const uint8_t>(result.data, result.len);
}

inline size_t MltBuffer::len() const {
    auto result = diplomat::capi::MltBuffer_len(this->AsFFI());
    return result;
}

inline const diplomat::capi::MltBuffer* MltBuffer::AsFFI() const {
    return reinterpret_cast<const diplomat::capi::MltBuffer*>(this);
}

inline diplomat::capi::MltBuffer* MltBuffer::AsFFI() {
    return reinterpret_cast<diplomat::capi::MltBuffer*>(this);
}

inline const MltBuffer* MltBuffer::FromFFI(const diplomat::capi::MltBuffer* ptr) {
    return reinterpret_cast<const MltBuffer*>(ptr);
}

inline MltBuffer* MltBuffer::FromFFI(diplomat::capi::MltBuffer* ptr) {
    return reinterpret_cast<MltBuffer*>(ptr);
}

inline void MltBuffer::operator delete(void* ptr) {
    diplomat::capi::MltBuffer_destroy(reinterpret_cast<diplomat::capi::MltBuffer*>(ptr));
}

#endif // MltBuffer_HPP
