#pragma once

#include <string>
#include <unordered_map>

namespace mlt {

struct Geometry {};

class Feature {
public:
    using id_t = std::uint64_t;
    using PropertyMap = std::unordered_map<std::string, std::string>;

    Feature() = delete;
    Feature(const Feature&) = delete;
    Feature(Feature&&) = default;

    Feature(id_t ident_, Geometry geometry_, PropertyMap properties_)
        : ident(ident_),
          geometry(std::move(geometry_)),
          properties(std::move(properties_)) {}

    id_t getID() const { return ident; }
    const Geometry& getGeometry() const { return geometry; }
    const PropertyMap getProperties() const { return properties; }

private:
    id_t ident;
    Geometry geometry;
    PropertyMap properties;
};

} // namespace mlt
