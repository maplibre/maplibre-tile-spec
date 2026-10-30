#pragma once

#include <mlt/common.hpp>
#include <mlt/geometry.hpp>
#include <mlt/tile.hpp>
#include <mlt/util/noncopyable.hpp>

#include <cstddef>
#include <memory>

namespace mlt {

class Decoder : public util::noncopyable {
public:
    using Geometry = geometry::Geometry;
    using GeometryFactory = geometry::GeometryFactory;

    Decoder();
    explicit Decoder(std::unique_ptr<GeometryFactory>&&);
    ~Decoder() noexcept;
    Decoder(Decoder&&) = delete;
    Decoder& operator=(Decoder&&) = delete;

    /// Limit the memory used for decoding a tile, in bytes, when it comes from the sizes in the tile.
    /// This protects against tiles that are small, but declare enormous amounts of data.
    /// The default is 64 MiB, the same as for the Rust decoder.
    void setMemoryLimit(std::size_t bytes) noexcept;

    /// Decode a tile
    /// @throws std::runtime_error if the tile is invalid or exceeds the memory limit
    MapLibreTile decode(DataView);
    MapLibreTile decode(BufferStream);

private:
    struct Impl;
    std::unique_ptr<Impl> impl;
};

} // namespace mlt
