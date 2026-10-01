#pragma once

#include <mlt/tile.hpp>
#include <nlohmann/json.hpp>

/// Test utility: convert an MLT tile to a flat GeoJSON FeatureCollection with raw tile-space
/// coordinates. Layer name and extent are stored as `_layer` and `_extent` in each feature's
/// properties. This matches the format used by test/synthetic expected JSON files.
namespace mlt::test {

nlohmann::json toFeatureCollection(const MapLibreTile& tile);

} // namespace mlt::test
