#pragma once

#include <memory>
#include <string>
#include <unordered_map>

namespace mlt {

class Geometry;

class Feature {
public:
    using id_t = std::uint64_t;
    using extent_t = std::uint32_t;
    using PropertyMap = std::unordered_map<std::string, std::string>;

    Feature() = delete;
    Feature(const Feature&) = delete;
    Feature(Feature&&) noexcept = default;
    Feature(id_t, extent_t, std::unique_ptr<Geometry>&&, PropertyMap) noexcept(false);
    ~Feature();

    id_t getID() const noexcept { return ident; }
    extent_t getExtent() const noexcept { return extent; }
    const Geometry& getGeometry() const noexcept { return *geometry; }
    const PropertyMap& getProperties() const noexcept { return properties; }

private:
    id_t ident;
    extent_t extent;
    std::unique_ptr<Geometry> geometry;
    PropertyMap properties;
};

} // namespace mlt
