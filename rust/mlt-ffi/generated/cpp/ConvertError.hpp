#ifndef ConvertError_HPP
#define ConvertError_HPP

#include "ConvertError.d.hpp"

#include "ConvertErrorKind.hpp"
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

diplomat::capi::ConvertErrorKind ConvertError_kind(const diplomat::capi::ConvertError* self);

void ConvertError_message(const diplomat::capi::ConvertError* self, diplomat::capi::DiplomatWrite* write);

void ConvertError_destroy(ConvertError* self);

} // extern "C"
} // namespace capi
} // namespace diplomat

inline ConvertErrorKind ConvertError::kind() const {
    auto result = diplomat::capi::ConvertError_kind(this->AsFFI());
    return ConvertErrorKind::FromFFI(result);
}

inline std::string ConvertError::message() const {
    std::string output;
    diplomat::capi::DiplomatWrite write = diplomat::WriteFromString(output);
    diplomat::capi::ConvertError_message(this->AsFFI(), &write);
    return output;
}
template <typename W>
inline void ConvertError::message_write(W& writeable) const {
    diplomat::capi::DiplomatWrite write = diplomat::WriteTrait<W>::Construct(writeable);
    diplomat::capi::ConvertError_message(this->AsFFI(), &write);
}

inline const diplomat::capi::ConvertError* ConvertError::AsFFI() const {
    return reinterpret_cast<const diplomat::capi::ConvertError*>(this);
}

inline diplomat::capi::ConvertError* ConvertError::AsFFI() {
    return reinterpret_cast<diplomat::capi::ConvertError*>(this);
}

inline const ConvertError* ConvertError::FromFFI(const diplomat::capi::ConvertError* ptr) {
    return reinterpret_cast<const ConvertError*>(ptr);
}

inline ConvertError* ConvertError::FromFFI(diplomat::capi::ConvertError* ptr) {
    return reinterpret_cast<ConvertError*>(ptr);
}

inline void ConvertError::operator delete(void* ptr) {
    diplomat::capi::ConvertError_destroy(reinterpret_cast<diplomat::capi::ConvertError*>(ptr));
}

#endif // ConvertError_HPP
