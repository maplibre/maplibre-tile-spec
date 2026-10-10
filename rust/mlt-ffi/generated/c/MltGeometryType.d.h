#ifndef MltGeometryType_D_H
#define MltGeometryType_D_H

#include "diplomat_runtime.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

typedef enum MltGeometryType {
    MltGeometryType_Point = 0,
    MltGeometryType_LineString = 1,
    MltGeometryType_Polygon = 2,
    MltGeometryType_MultiPoint = 3,
    MltGeometryType_MultiLineString = 4,
    MltGeometryType_MultiPolygon = 5,
} MltGeometryType;

typedef struct MltGeometryType_option {
    union {
        MltGeometryType ok;
    };
    bool is_ok;
} MltGeometryType_option;

#endif // MltGeometryType_D_H
