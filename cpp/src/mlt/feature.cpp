#include <mlt/feature.hpp>

#include <mlt/geometry.hpp>

#include <stdexcept>

namespace mlt {

Feature::Feature(id_t ident_,
                 std::unique_ptr<Geometry>&& geometry_,
                 PropertyMap properties_,
                 const PropertyVecMap& propertyMap_,
                 std::size_t propertyIndex_)
    : ident(ident_),
      geometry(std::move(geometry_)),
      properties(std::move(properties_)),
      propertyMap(propertyMap_),
      propertyIndex(propertyIndex_) {
    if (!geometry) {
        throw std::runtime_error("missing geometry");
    }
}

Feature::~Feature() noexcept = default;

const Property& Feature::getProperty(std::string_view key) const {
    static Property prop;
    return prop;
}

} // namespace mlt
