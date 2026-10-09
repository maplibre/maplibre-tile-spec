#ifndef ConvertErrorKind_HPP
#define ConvertErrorKind_HPP

#include "ConvertErrorKind.d.hpp"

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

} // namespace capi
} // namespace

inline diplomat::capi::ConvertErrorKind ConvertErrorKind::AsFFI() const
{
    return static_cast<diplomat::capi::ConvertErrorKind>(value);
}

inline ConvertErrorKind ConvertErrorKind::FromFFI(diplomat::capi::ConvertErrorKind c_enum)
{
    switch (c_enum) {
    case diplomat::capi::ConvertErrorKind_InvalidInput:
    case diplomat::capi::ConvertErrorKind_EncodingFailed:
        return static_cast<ConvertErrorKind::Value>(c_enum);
    default:
        std::abort();
    }
}
#endif // ConvertErrorKind_HPP
