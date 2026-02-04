#pragma once

#include <mlt/common.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/noncopyable.hpp>

#include <cstdint>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <variant>
#include <vector>

namespace mlt {

struct EncoderConfig {
    bool useFastPfor = false;
    bool includeIds = true;
};

class Encoder : public util::noncopyable {
public:
    using GeometryType = metadata::tileset::GeometryType;

    struct Vertex {
        std::int32_t x;
        std::int32_t y;
    };

    using PropertyValue = std::variant<
        bool,
        std::int32_t,
        std::int64_t,
        std::uint32_t,
        std::uint64_t,
        float,
        double,
        std::string>;

    struct Geometry {
        GeometryType type;
        /// Flat vertex list for simple geometries.
        std::vector<Vertex> coordinates;
        /// For multi-geometries: each sub-geometry's vertices.
        std::vector<std::vector<Vertex>> parts;
        /// For polygons: ring sizes (exterior ring first, then interior rings).
        std::vector<std::uint32_t> ringSizes;
        /// For multi-polygons: per-polygon ring sizes.
        std::vector<std::vector<std::uint32_t>> partRingSizes;
    };

    struct Feature {
        std::uint64_t id = 0;
        Geometry geometry;
        std::map<std::string, PropertyValue> properties;
    };

    struct Layer {
        std::string name;
        std::uint32_t extent = 4096;
        std::vector<Feature> features;
    };

    Encoder();
    ~Encoder() noexcept;

    Encoder(Encoder&&) = delete;
    Encoder& operator=(Encoder&&) = delete;

    /// Encode a complete MLT tile from a set of layers.
    std::vector<std::uint8_t> encode(const std::vector<Layer>& layers,
                                     const EncoderConfig& config = {});

private:
    struct Impl;
    std::unique_ptr<Impl> impl;
};

} // namespace mlt
