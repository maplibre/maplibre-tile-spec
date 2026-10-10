#ifndef MltGeometryType_D_HPP
#define MltGeometryType_D_HPP

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
enum MltGeometryType {
    MltGeometryType_Point = 0,
    MltGeometryType_LineString = 1,
    MltGeometryType_Polygon = 2,
    MltGeometryType_MultiPoint = 3,
    MltGeometryType_MultiLineString = 4,
    MltGeometryType_MultiPolygon = 5,
};

typedef struct MltGeometryType_option {
    union {
        MltGeometryType ok;
    };
    bool is_ok;
} MltGeometryType_option;
} // namespace capi
} // namespace diplomat

/**
 * The geometry type of one feature.
 */
class MltGeometryType {
public:
    enum Value {
        Point = 0,
        LineString = 1,
        Polygon = 2,
        MultiPoint = 3,
        MultiLineString = 4,
        MultiPolygon = 5,
    };

    MltGeometryType()
        : value(Value::Point) {}

    // Implicit conversions between enum and ::Value
    constexpr MltGeometryType(Value v)
        : value(v) {}
    constexpr operator Value() const { return value; }
    // Prevent usage as boolean value
    explicit operator bool() const = delete;

    inline diplomat::capi::MltGeometryType AsFFI() const;
    inline static MltGeometryType FromFFI(diplomat::capi::MltGeometryType c_enum);

private:
    Value value;
};

#endif // MltGeometryType_D_HPP
