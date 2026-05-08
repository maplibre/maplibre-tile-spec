#ifndef MltBuffer_D_HPP
#define MltBuffer_D_HPP

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <memory>
#include <functional>
#include <optional>
#include <cstdlib>
#include "diplomat_runtime.hpp"

namespace diplomat {
namespace capi {
struct MltBuffer;
} // namespace capi
} // namespace diplomat

/**
 * Owned byte buffer returned from conversion functions.
 *
 * The caller borrows the contents via {@link as_bytes}(MltBuffer::as_bytes)
 * and the buffer is freed when the handle is dropped.
 */
class MltBuffer {
public:
    /**
     * Borrow the contents as a byte slice.
     */
    inline diplomat::span<const uint8_t> as_bytes() const;

    /**
     * Number of bytes in the buffer.
     */
    inline size_t len() const;

    inline const diplomat::capi::MltBuffer* AsFFI() const;
    inline diplomat::capi::MltBuffer* AsFFI();
    inline static const MltBuffer* FromFFI(const diplomat::capi::MltBuffer* ptr);
    inline static MltBuffer* FromFFI(diplomat::capi::MltBuffer* ptr);
    inline static void operator delete(void* ptr);

private:
    MltBuffer() = delete;
    MltBuffer(const MltBuffer&) = delete;
    MltBuffer(MltBuffer&&) noexcept = delete;
    MltBuffer operator=(const MltBuffer&) = delete;
    MltBuffer operator=(MltBuffer&&) noexcept = delete;
    static void operator delete[](void*, size_t) = delete;
};

#endif // MltBuffer_D_HPP
