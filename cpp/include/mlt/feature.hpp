#pragma once

#include <mlt/properties.hpp>
#include <mlt/util/noncopyable.hpp>

#include <memory>

namespace mlt {

class Geometry;

class Feature : public util::noncopyable {
public:
    using id_t = std::uint64_t;
    using extent_t = std::uint32_t;

    Feature() = delete;
    Feature(Feature&&) noexcept = default;
    Feature& operator=(Feature&&) = delete;

    /// Construct a feature
    /// @param id Feature identifier
    /// @param extent Tile extent
    /// @param geometry Feature geometry, required
    /// @param properties Feature properties, optional
    /// @throws `std::runtime_error` Missing geometry
    /// TODO: use a non-nullable type for the polymorphic geometry object
    Feature(id_t id, std::unique_ptr<Geometry>&& geometry, PropertyMap properties);

    ~Feature() noexcept;

    id_t getID() const noexcept { return ident; }
    const Geometry& getGeometry() const noexcept { return *geometry; }
    const PropertyMap& getProperties() const noexcept { return properties; }

private:
    id_t ident;
    extent_t extent;
    std::unique_ptr<Geometry> geometry;
    PropertyMap properties;
};

} // namespace mlt
