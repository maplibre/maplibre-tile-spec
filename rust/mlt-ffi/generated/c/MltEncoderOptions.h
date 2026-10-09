#ifndef MltEncoderOptions_H
#define MltEncoderOptions_H

#include "diplomat_runtime.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "MltWireVersion.d.h"

#include "MltEncoderOptions.d.h"

MltEncoderOptions* MltEncoderOptions_new(void);

void MltEncoderOptions_set_tessellate(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_attempt_spatial_morton_sort(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_attempt_spatial_hilbert_sort(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_attempt_id_sort(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_fsst(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_fastpfor(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_shared_dict(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_wire_version(MltEncoderOptions* self, MltWireVersion version);

void MltEncoderOptions_set_allow_triangles_only(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_delta2(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_float_dict(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_float_alp(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_packed_dict_codes(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_rans_vertices(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_set_allow_row_shapes(MltEncoderOptions* self, bool enabled);

void MltEncoderOptions_destroy(MltEncoderOptions* self);

#endif // MltEncoderOptions_H
