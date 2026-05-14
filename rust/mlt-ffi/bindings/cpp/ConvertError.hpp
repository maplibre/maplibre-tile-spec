#ifndef ConvertError_HPP
#define ConvertError_HPP

#include "ConvertError.d.hpp"

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
