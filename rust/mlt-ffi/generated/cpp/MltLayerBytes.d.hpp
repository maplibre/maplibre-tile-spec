#ifndef MltLayerBytes_D_HPP
#define MltLayerBytes_D_HPP

#include "diplomat_runtime.hpp"
#include <cstdlib>
#include <functional>
#include <memory>
#include <optional>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

namespace diplomat {
namespace capi {
struct MltLayerBytes {
    uint32_t size;
    uint32_t geometry;
    uint32_t properties;
    uint32_t ids;
    uint32_t metadata;
};

typedef struct MltLayerBytes_option {
    union {
        MltLayerBytes ok;
    };
    bool is_ok;
} MltLayerBytes_option;
} // namespace capi
} // namespace diplomat

/**
 * The bytes one layer spends, by what they hold.
 */
struct MltLayerBytes {
    uint32_t size;
    uint32_t geometry;
    uint32_t properties;
    uint32_t ids;
    uint32_t metadata;

    inline diplomat::capi::MltLayerBytes AsFFI() const;
    inline static MltLayerBytes FromFFI(diplomat::capi::MltLayerBytes c_struct);
};

#endif // MltLayerBytes_D_HPP
