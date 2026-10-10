#ifndef MltGeometryType_HPP
#define MltGeometryType_HPP

#include "MltGeometryType.d.hpp"

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

inline diplomat::capi::MltGeometryType MltGeometryType::AsFFI() const
{
    return static_cast<diplomat::capi::MltGeometryType>(value);
}

inline MltGeometryType MltGeometryType::FromFFI(diplomat::capi::MltGeometryType c_enum)
{
    switch (c_enum) {
    case diplomat::capi::MltGeometryType_Point:
    case diplomat::capi::MltGeometryType_LineString:
    case diplomat::capi::MltGeometryType_Polygon:
    case diplomat::capi::MltGeometryType_MultiPoint:
    case diplomat::capi::MltGeometryType_MultiLineString:
    case diplomat::capi::MltGeometryType_MultiPolygon:
        return static_cast<MltGeometryType::Value>(c_enum);
    default:
        std::abort();
    }
}
#endif // MltGeometryType_HPP
