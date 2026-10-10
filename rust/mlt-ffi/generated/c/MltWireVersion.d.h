#ifndef MltWireVersion_D_H
#define MltWireVersion_D_H

#include "diplomat_runtime.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

typedef enum MltWireVersion {
    MltWireVersion_V01 = 0,
    MltWireVersion_V02 = 1,
} MltWireVersion;

typedef struct MltWireVersion_option {
    union {
        MltWireVersion ok;
    };
    bool is_ok;
} MltWireVersion_option;

#endif // MltWireVersion_D_H
