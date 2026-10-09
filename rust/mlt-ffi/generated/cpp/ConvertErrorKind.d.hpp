#ifndef ConvertErrorKind_D_HPP
#define ConvertErrorKind_D_HPP

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
    enum ConvertErrorKind {
        ConvertErrorKind_InvalidInput = 0,
        ConvertErrorKind_EncodingFailed = 1,
    };

    typedef struct ConvertErrorKind_option {
        union {
            ConvertErrorKind ok;
        };
        bool is_ok;
    } ConvertErrorKind_option;
} // namespace capi
} // namespace

/**
 * Which stage of a conversion failed.
 */
class ConvertErrorKind {
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

    ConvertErrorKind()
        : value(Value::InvalidInput)
    {
    }

    // Implicit conversions between enum and ::Value
    constexpr ConvertErrorKind(Value v)
        : value(v)
    {
    }
    constexpr operator Value() const { return value; }
    // Prevent usage as boolean value
    explicit operator bool() const = delete;

    inline diplomat::capi::ConvertErrorKind AsFFI() const;
    inline static ConvertErrorKind FromFFI(diplomat::capi::ConvertErrorKind c_enum);

private:
    Value value;
};

#endif // ConvertErrorKind_D_HPP
