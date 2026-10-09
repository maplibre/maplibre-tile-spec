#ifndef MltWireVersion_HPP
#define MltWireVersion_HPP

#include "MltWireVersion.d.hpp"

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

inline diplomat::capi::MltWireVersion MltWireVersion::AsFFI() const {
    return static_cast<diplomat::capi::MltWireVersion>(value);
}

inline MltWireVersion MltWireVersion::FromFFI(diplomat::capi::MltWireVersion c_enum) {
    switch (c_enum) {
        case diplomat::capi::MltWireVersion_V01:
        case diplomat::capi::MltWireVersion_V02:
            return static_cast<MltWireVersion::Value>(c_enum);
        default:
            std::abort();
    }
}
#endif // MltWireVersion_HPP
