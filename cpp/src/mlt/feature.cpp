#include <mlt/feature.hpp>

#include <mlt/geometry.hpp>

#include <stdexcept>

namespace mlt {

Feature::Feature(id_t ident_, std::unique_ptr<Geometry>&& geometry_, PropertyMap properties_)
    : ident(ident_),
      geometry(std::move(geometry_)),
      properties(std::move(properties_)) {
    if (!geometry) {
        throw std::runtime_error("missing geometry");
    }
}

Feature::~Feature() noexcept = default;

} // namespace mlt
