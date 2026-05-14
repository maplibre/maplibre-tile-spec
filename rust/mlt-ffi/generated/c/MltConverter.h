#ifndef MltConverter_H
#define MltConverter_H

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include "diplomat_runtime.h"

#include "ConvertError.d.h"
#include "MltBuffer.d.h"
#include "MltEncoderOptions.d.h"

#include "MltConverter.d.h"

typedef struct MltConverter_mlt_to_mvt_result {
    union {
        MltBuffer* ok;
        ConvertError err;
    };
    bool is_ok;
} MltConverter_mlt_to_mvt_result;
MltConverter_mlt_to_mvt_result MltConverter_mlt_to_mvt(DiplomatU8View mlt);

typedef struct MltConverter_mvt_to_mlt_result {
    union {
        MltBuffer* ok;
        ConvertError err;
    };
    bool is_ok;
} MltConverter_mvt_to_mlt_result;
MltConverter_mvt_to_mlt_result MltConverter_mvt_to_mlt(DiplomatU8View mvt, const MltEncoderOptions* options);

void MltConverter_destroy(MltConverter* self);

#endif // MltConverter_H
