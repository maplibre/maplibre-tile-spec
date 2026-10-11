#ifndef MltLayerBytes_HPP
#define MltLayerBytes_HPP

#include "MltLayerBytes.d.hpp"

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
namespace capi {} // namespace capi
} // namespace diplomat

inline diplomat::capi::MltLayerBytes MltLayerBytes::AsFFI() const {
    return diplomat::capi::MltLayerBytes{
        /* .size = */ size,
        /* .geometry = */ geometry,
        /* .properties = */ properties,
        /* .ids = */ ids,
        /* .metadata = */ metadata,
    };
}

inline MltLayerBytes MltLayerBytes::FromFFI(diplomat::capi::MltLayerBytes c_struct) {
    return MltLayerBytes{
        /* .size = */ c_struct.size,
        /* .geometry = */ c_struct.geometry,
        /* .properties = */ c_struct.properties,
        /* .ids = */ c_struct.ids,
        /* .metadata = */ c_struct.metadata,
    };
}

#endif // MltLayerBytes_HPP
