#ifndef MltTileStats_H
#define MltTileStats_H

#include "diplomat_runtime.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "ConvertError.d.h"
#include "MltLayerBytes.d.h"

#include "MltTileStats.d.h"

typedef struct MltTileStats_from_bytes_result {
    union {
        MltTileStats* ok;
        ConvertError* err;
    };
    bool is_ok;
} MltTileStats_from_bytes_result;
MltTileStats_from_bytes_result MltTileStats_from_bytes(DiplomatU8View mlt);

size_t MltTileStats_layer_count(const MltTileStats* self);

void MltTileStats_layer_name(const MltTileStats* self, size_t i, DiplomatWrite* write);

typedef struct MltTileStats_layer_bytes_result {
    union {
        MltLayerBytes ok;
    };
    bool is_ok;
} MltTileStats_layer_bytes_result;
MltTileStats_layer_bytes_result MltTileStats_layer_bytes(const MltTileStats* self, size_t i);

void MltTileStats_destroy(MltTileStats* self);

#endif // MltTileStats_H
