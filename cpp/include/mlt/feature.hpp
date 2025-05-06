#pragma once

#include <mlt/properties.hpp>
#include <mlt/util/noncopyable.hpp>

#include <memory>

namespace mlt {

namespace geometry {
class Geometry;
}

class Feature : public util::noncopyable {
public:
    using id_t = std::uint64_t;
    using extent_t = std::uint32_t;
    using Geometry = geometry::Geometry;

    Feature() = delete;
    Feature(Feature&&) noexcept = default;
    Feature& operator=(Feature&&) = delete;

    /// Construct a feature
    /// @param id Feature identifier
    /// @param extent Tile extent
    /// @param geometry Feature geometry, required
    /// @param properties Feature properties, optional
    /// @throws `std::runtime_error` Missing geometry
    Feature(id_t, std::unique_ptr<Geometry>&&, PropertyMap, const PropertyVecMap&, std::size_t propertyIndex_);

    ~Feature() noexcept;

    id_t getID() const noexcept { return ident; }
    const Geometry& getGeometry() const noexcept { return *geometry; }
    const PropertyMap& getProperties() const noexcept { return properties; }
    const Property& getProperty(std::string_view key) const;

private:
    id_t ident;
    extent_t extent;
    std::unique_ptr<Geometry> geometry;
    PropertyMap properties;

    const PropertyVecMap& propertyMap; // owned by the containing layer
    std::size_t propertyIndex;         // index of the property in the layer
};

} // namespace mlt
