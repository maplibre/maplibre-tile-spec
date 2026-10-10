#ifndef MltLayerBytes_D_H
#define MltLayerBytes_D_H

#include "diplomat_runtime.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

typedef struct MltLayerBytes {
    uint32_t size;
    uint32_t geometry;
    uint32_t properties;
    uint32_t ids;
    uint32_t metadata;
} MltLayerBytes;

typedef struct MltLayerBytes_option {
    union {
        MltLayerBytes ok;
    };
    bool is_ok;
} MltLayerBytes_option;

#endif // MltLayerBytes_D_H
