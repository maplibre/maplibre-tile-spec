#include <feature.hpp>

#include <geometry.hpp>

namespace mlt {

Feature::Feature(id_t ident_,
                 extent_t extent_,
                 std::unique_ptr<Geometry>&& geometry_,
                 PropertyMap properties_) noexcept(false)
    : ident(ident_),
      extent(extent_),
      geometry(std::move(geometry_)),
      properties(std::move(properties_)) {}

Feature::~Feature() = default;

} // namespace mlt
