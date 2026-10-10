#ifndef MltLayerBuilder_H
#define MltLayerBuilder_H

#include "diplomat_runtime.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "ConvertError.d.h"
#include "MltBuffer.d.h"
#include "MltEncoderOptions.d.h"
#include "MltMvtGeometryType.d.h"

#include "MltLayerBuilder.d.h"

typedef struct MltLayerBuilder_new_result {
    union {
        MltLayerBuilder* ok;
        ConvertError* err;
    };
    bool is_ok;
} MltLayerBuilder_new_result;
MltLayerBuilder_new_result MltLayerBuilder_new(DiplomatStringView name, uint32_t extent);

typedef struct MltLayerBuilder_reset_result {
    union {
        ConvertError* err;
    };
    bool is_ok;
} MltLayerBuilder_reset_result;
MltLayerBuilder_reset_result MltLayerBuilder_reset(MltLayerBuilder* self, DiplomatStringView name, uint32_t extent);

uint32_t MltLayerBuilder_add_property(MltLayerBuilder* self, DiplomatStringView name);

typedef struct MltLayerBuilder_begin_mvt_feature_result {
    union {
        ConvertError* err;
    };
    bool is_ok;
} MltLayerBuilder_begin_mvt_feature_result;
MltLayerBuilder_begin_mvt_feature_result MltLayerBuilder_begin_mvt_feature(MltLayerBuilder* self,
                                                                           MltMvtGeometryType geometry,
                                                                           DiplomatU32View commands,
                                                                           OptionU64 id);

typedef struct MltLayerBuilder_set_bool_result {
    union {
        ConvertError* err;
    };
    bool is_ok;
} MltLayerBuilder_set_bool_result;
MltLayerBuilder_set_bool_result MltLayerBuilder_set_bool(MltLayerBuilder* self, uint32_t key, bool value);

typedef struct MltLayerBuilder_set_i64_result {
    union {
        ConvertError* err;
    };
    bool is_ok;
} MltLayerBuilder_set_i64_result;
MltLayerBuilder_set_i64_result MltLayerBuilder_set_i64(MltLayerBuilder* self, uint32_t key, int64_t value);

typedef struct MltLayerBuilder_set_f32_result {
    union {
        ConvertError* err;
    };
    bool is_ok;
} MltLayerBuilder_set_f32_result;
MltLayerBuilder_set_f32_result MltLayerBuilder_set_f32(MltLayerBuilder* self, uint32_t key, float value);

typedef struct MltLayerBuilder_set_f64_result {
    union {
        ConvertError* err;
    };
    bool is_ok;
} MltLayerBuilder_set_f64_result;
MltLayerBuilder_set_f64_result MltLayerBuilder_set_f64(MltLayerBuilder* self, uint32_t key, double value);

typedef struct MltLayerBuilder_set_str_result {
    union {
        ConvertError* err;
    };
    bool is_ok;
} MltLayerBuilder_set_str_result;
MltLayerBuilder_set_str_result MltLayerBuilder_set_str(MltLayerBuilder* self, uint32_t key, DiplomatStringView value);

typedef struct MltLayerBuilder_encode_into_result {
    union {
        ConvertError* err;
    };
    bool is_ok;
} MltLayerBuilder_encode_into_result;
MltLayerBuilder_encode_into_result MltLayerBuilder_encode_into(const MltLayerBuilder* self,
                                                               const MltEncoderOptions* options,
                                                               MltBuffer* out);

void MltLayerBuilder_destroy(MltLayerBuilder* self);

#endif // MltLayerBuilder_H
