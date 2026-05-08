#ifndef ConvertError_D_H
#define ConvertError_D_H

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include "diplomat_runtime.h"

typedef enum ConvertError {
    ConvertError_InvalidInput = 0,
    ConvertError_EncodingFailed = 1,
} ConvertError;

typedef struct ConvertError_option {
    union {
        ConvertError ok;
    };
    bool is_ok;
} ConvertError_option;

#endif // ConvertError_D_H
