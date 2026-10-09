#ifndef ConvertError_H
#define ConvertError_H

#include "diplomat_runtime.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "ConvertErrorKind.d.h"

#include "ConvertError.d.h"

ConvertErrorKind ConvertError_kind(const ConvertError* self);

void ConvertError_message(const ConvertError* self, DiplomatWrite* write);

void ConvertError_destroy(ConvertError* self);

#endif // ConvertError_H
