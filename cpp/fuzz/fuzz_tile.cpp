// Fuzz the tile decoder, which is given the contents of a whole tile.

#include <mlt/decoder.hpp>
#include <mlt/tile.hpp>

#if MLT_WITH_JSON
#include <mlt/json.hpp>
#endif

#include <cstddef>
#include <cstdint>
#include <stdexcept>

extern "C" int LLVMFuzzerTestOneInput(const std::uint8_t* data, std::size_t size) {
    try {
        mlt::Decoder decoder;
        const auto tile = decoder.decode({reinterpret_cast<const char*>(data), size});
#if MLT_WITH_JSON
        // Visit everything in the tile, which includes geometries and properties
        (void)mlt::json::toJSON(tile, {.x = 0, .y = 0, .z = 0}, true);
#else
        (void)tile;
#endif
    } catch (const std::runtime_error&) {
        // Rejecting invalid tiles is fine. Any other exception, such as running out of memory, is not.
    }
    return 0;
}
