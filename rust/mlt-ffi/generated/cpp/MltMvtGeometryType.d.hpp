#ifndef MltMvtGeometryType_D_HPP
#define MltMvtGeometryType_D_HPP

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
enum MltMvtGeometryType {
    MltMvtGeometryType_Point = 0,
    MltMvtGeometryType_LineString = 1,
    MltMvtGeometryType_Polygon = 2,
};

typedef struct MltMvtGeometryType_option {
    union {
        MltMvtGeometryType ok;
    };
    bool is_ok;
} MltMvtGeometryType_option;
} // namespace capi
} // namespace diplomat

/**
 * The geometry type of an MVT feature, which also covers its multi variant.
 */
class MltMvtGeometryType {
public:
    enum Value {
        Point = 0,
        LineString = 1,
        Polygon = 2,
    };

    MltMvtGeometryType()
        : value(Value::Point) {}

    // Implicit conversions between enum and ::Value
    constexpr MltMvtGeometryType(Value v)
        : value(v) {}
    constexpr operator Value() const { return value; }
    // Prevent usage as boolean value
    explicit operator bool() const = delete;

    inline diplomat::capi::MltMvtGeometryType AsFFI() const;
    inline static MltMvtGeometryType FromFFI(diplomat::capi::MltMvtGeometryType c_enum);

private:
    Value value;
};

#endif // MltMvtGeometryType_D_HPP
