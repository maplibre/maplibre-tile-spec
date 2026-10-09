#ifndef ConvertError_D_HPP
#define ConvertError_D_HPP

#include "diplomat_runtime.hpp"
#include <cstdlib>
#include <functional>
#include <memory>
#include <optional>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

class ConvertErrorKind;

namespace diplomat {
namespace capi {
struct ConvertError;
} // namespace capi
} // namespace diplomat

/**
 * Error returned by FFI conversion functions.
 */
class ConvertError {
public:
    /**
     * The stage that failed.
     */
    inline ConvertErrorKind kind() const;

    /**
     * Human-readable cause.
     */
    inline std::string message() const;
    template <typename W>
    inline void message_write(W& writeable_output) const;

    inline const diplomat::capi::ConvertError* AsFFI() const;
    inline diplomat::capi::ConvertError* AsFFI();
    inline static const ConvertError* FromFFI(const diplomat::capi::ConvertError* ptr);
    inline static ConvertError* FromFFI(diplomat::capi::ConvertError* ptr);
    inline static void operator delete(void* ptr);

private:
    ConvertError() = delete;
    ConvertError(const ConvertError&) = delete;
    ConvertError(ConvertError&&) noexcept = delete;
    ConvertError operator=(const ConvertError&) = delete;
    ConvertError operator=(ConvertError&&) noexcept = delete;
    static void operator delete[](void*, size_t) = delete;
};

#endif // ConvertError_D_HPP
