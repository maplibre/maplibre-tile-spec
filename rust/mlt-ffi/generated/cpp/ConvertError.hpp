#ifndef ConvertError_HPP
#define ConvertError_HPP

#include "ConvertError.d.hpp"

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
namespace capi {} // namespace capi
} // namespace diplomat

inline diplomat::capi::ConvertError ConvertError::AsFFI() const {
    return static_cast<diplomat::capi::ConvertError>(value);
}

inline ConvertError ConvertError::FromFFI(diplomat::capi::ConvertError c_enum) {
    switch (c_enum) {
        case diplomat::capi::ConvertError_InvalidInput:
        case diplomat::capi::ConvertError_EncodingFailed:
            return static_cast<ConvertError::Value>(c_enum);
        default:
            std::abort();
    }
}
#endif // ConvertError_HPP
