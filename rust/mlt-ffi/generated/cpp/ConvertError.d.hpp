#ifndef ConvertError_D_HPP
#define ConvertError_D_HPP

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
enum ConvertError {
    ConvertError_InvalidInput = 0,
    ConvertError_EncodingFailed = 1,
};

typedef struct ConvertError_option {
    union {
        ConvertError ok;
    };
    bool is_ok;
} ConvertError_option;
} // namespace capi
} // namespace diplomat

/**
 * Error type returned by FFI conversion functions.
 */
class ConvertError {
public:
    enum Value {
        /**
         * Input bytes could not be parsed or decoded.
         */
        InvalidInput = 0,
        /**
         * Encoding failed.
         */
        EncodingFailed = 1,
    };

    ConvertError()
        : value(Value::InvalidInput) {}

    // Implicit conversions between enum and ::Value
    constexpr ConvertError(Value v)
        : value(v) {}
    constexpr operator Value() const { return value; }
    // Prevent usage as boolean value
    explicit operator bool() const = delete;

    inline diplomat::capi::ConvertError AsFFI() const;
    inline static ConvertError FromFFI(diplomat::capi::ConvertError c_enum);

private:
    Value value;
};

#endif // ConvertError_D_HPP
