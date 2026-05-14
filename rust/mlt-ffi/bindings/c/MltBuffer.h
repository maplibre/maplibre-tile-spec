#ifndef MltBuffer_H
#define MltBuffer_H

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include "diplomat_runtime.h"

#include "MltBuffer.d.h"

DiplomatU8View MltBuffer_as_bytes(const MltBuffer* self);

size_t MltBuffer_len(const MltBuffer* self);

void MltBuffer_destroy(MltBuffer* self);

#endif // MltBuffer_H
