#ifndef MltMvtGeometryType_HPP
#define MltMvtGeometryType_HPP

#include "MltMvtGeometryType.d.hpp"

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

inline diplomat::capi::MltMvtGeometryType MltMvtGeometryType::AsFFI() const
{
    return static_cast<diplomat::capi::MltMvtGeometryType>(value);
}

inline MltMvtGeometryType MltMvtGeometryType::FromFFI(diplomat::capi::MltMvtGeometryType c_enum)
{
    switch (c_enum) {
    case diplomat::capi::MltMvtGeometryType_Point:
    case diplomat::capi::MltMvtGeometryType_LineString:
    case diplomat::capi::MltMvtGeometryType_Polygon:
        return static_cast<MltMvtGeometryType::Value>(c_enum);
    default:
        std::abort();
    }
}
#endif // MltMvtGeometryType_HPP
