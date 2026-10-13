#ifndef MltBuffer_H
#define MltBuffer_H

#include "diplomat_runtime.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "MltBuffer.d.h"

MltBuffer* MltBuffer_new(void);

void MltBuffer_clear(MltBuffer* self);

DiplomatU8View MltBuffer_as_bytes(const MltBuffer* self);

size_t MltBuffer_len(const MltBuffer* self);

void MltBuffer_destroy(MltBuffer* self);

#endif // MltBuffer_H
