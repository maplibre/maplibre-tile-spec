#ifndef MltWireVersion_D_HPP
#define MltWireVersion_D_HPP

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
    enum MltWireVersion {
        MltWireVersion_V01 = 0,
        MltWireVersion_V02 = 1,
    };

    typedef struct MltWireVersion_option {
        union {
            MltWireVersion ok;
        };
        bool is_ok;
    } MltWireVersion_option;
} // namespace capi
} // namespace

/**
 * The wire format an encoded layer uses.
 */
class MltWireVersion {
public:
    enum Value {
        /**
         * Tag `0x01`, the stable v1 format.
         */
        V01 = 0,
        /**
         * Tag `0x02`, the experimental v2 format.
         */
        V02 = 1,
    };

    MltWireVersion()
        : value(Value::V01)
    {
    }

    // Implicit conversions between enum and ::Value
    constexpr MltWireVersion(Value v)
        : value(v)
    {
    }
    constexpr operator Value() const { return value; }
    // Prevent usage as boolean value
    explicit operator bool() const = delete;

    inline diplomat::capi::MltWireVersion AsFFI() const;
    inline static MltWireVersion FromFFI(diplomat::capi::MltWireVersion c_enum);

private:
    Value value;
};

#endif // MltWireVersion_D_HPP
