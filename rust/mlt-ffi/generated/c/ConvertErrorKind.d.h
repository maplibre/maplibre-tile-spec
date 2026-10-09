#ifndef ConvertErrorKind_D_H
#define ConvertErrorKind_D_H

#include "diplomat_runtime.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

typedef enum ConvertErrorKind {
    ConvertErrorKind_InvalidInput = 0,
    ConvertErrorKind_EncodingFailed = 1,
} ConvertErrorKind;

typedef struct ConvertErrorKind_option {
    union {
        ConvertErrorKind ok;
    };
    bool is_ok;
} ConvertErrorKind_option;

#endif // ConvertErrorKind_D_H
