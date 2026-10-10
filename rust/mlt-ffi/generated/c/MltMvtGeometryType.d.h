#ifndef MltMvtGeometryType_D_H
#define MltMvtGeometryType_D_H

#include "diplomat_runtime.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

typedef enum MltMvtGeometryType {
    MltMvtGeometryType_Point = 0,
    MltMvtGeometryType_LineString = 1,
    MltMvtGeometryType_Polygon = 2,
} MltMvtGeometryType;

typedef struct MltMvtGeometryType_option {
    union {
        MltMvtGeometryType ok;
    };
    bool is_ok;
} MltMvtGeometryType_option;

#endif // MltMvtGeometryType_D_H
