#ifndef MltConverter_HPP
#define MltConverter_HPP

#include "MltConverter.d.hpp"

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <memory>
#include <functional>
#include <optional>
#include <cstdlib>
#include "ConvertError.hpp"
#include "MltBuffer.hpp"
#include "MltEncoderOptions.hpp"
#include "diplomat_runtime.hpp"


namespace diplomat {
namespace capi {
    extern "C" {

    typedef struct MltConverter_mlt_to_mvt_result {union {diplomat::capi::MltBuffer* ok; diplomat::capi::ConvertError err;}; bool is_ok;} MltConverter_mlt_to_mvt_result;
    MltConverter_mlt_to_mvt_result MltConverter_mlt_to_mvt(diplomat::capi::DiplomatU8View mlt);

    typedef struct MltConverter_mvt_to_mlt_result {union {diplomat::capi::MltBuffer* ok; diplomat::capi::ConvertError err;}; bool is_ok;} MltConverter_mvt_to_mlt_result;
    MltConverter_mvt_to_mlt_result MltConverter_mvt_to_mlt(diplomat::capi::DiplomatU8View mvt, const diplomat::capi::MltEncoderOptions* options);

    void MltConverter_destroy(MltConverter* self);

    } // extern "C"
} // namespace capi
} // namespace

inline diplomat::result<std::unique_ptr<MltBuffer>, ConvertError> MltConverter::mlt_to_mvt(diplomat::span<const uint8_t> mlt) {
    auto result = diplomat::capi::MltConverter_mlt_to_mvt({mlt.data(), mlt.size()});
    return result.is_ok ? diplomat::result<std::unique_ptr<MltBuffer>, ConvertError>(diplomat::Ok<std::unique_ptr<MltBuffer>>(std::unique_ptr<MltBuffer>(MltBuffer::FromFFI(result.ok)))) : diplomat::result<std::unique_ptr<MltBuffer>, ConvertError>(diplomat::Err<ConvertError>(ConvertError::FromFFI(result.err)));
}

inline diplomat::result<std::unique_ptr<MltBuffer>, ConvertError> MltConverter::mvt_to_mlt(diplomat::span<const uint8_t> mvt, const MltEncoderOptions& options) {
    auto result = diplomat::capi::MltConverter_mvt_to_mlt({mvt.data(), mvt.size()},
        options.AsFFI());
    return result.is_ok ? diplomat::result<std::unique_ptr<MltBuffer>, ConvertError>(diplomat::Ok<std::unique_ptr<MltBuffer>>(std::unique_ptr<MltBuffer>(MltBuffer::FromFFI(result.ok)))) : diplomat::result<std::unique_ptr<MltBuffer>, ConvertError>(diplomat::Err<ConvertError>(ConvertError::FromFFI(result.err)));
}

inline const diplomat::capi::MltConverter* MltConverter::AsFFI() const {
    return reinterpret_cast<const diplomat::capi::MltConverter*>(this);
}

inline diplomat::capi::MltConverter* MltConverter::AsFFI() {
    return reinterpret_cast<diplomat::capi::MltConverter*>(this);
}

inline const MltConverter* MltConverter::FromFFI(const diplomat::capi::MltConverter* ptr) {
    return reinterpret_cast<const MltConverter*>(ptr);
}

inline MltConverter* MltConverter::FromFFI(diplomat::capi::MltConverter* ptr) {
    return reinterpret_cast<MltConverter*>(ptr);
}

inline void MltConverter::operator delete(void* ptr) {
    diplomat::capi::MltConverter_destroy(reinterpret_cast<diplomat::capi::MltConverter*>(ptr));
}


#endif // MltConverter_HPP
