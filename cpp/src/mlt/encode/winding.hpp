#pragma once

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <span>

namespace mlt::encoder {

/// Winds polygon rings as MVT does: an exterior ring has a positive area by the surveyor's formula in tile
/// coordinates, and a hole a negative one. That is clockwise on a tile whose y-axis points down.
/// A ring that encloses no area keeps its winding.
class PolygonWinding {
public:
    /// Wind the rings of one polygon in place.
    /// @param vertices The vertices of all the rings, one after the other.
    /// @param ringSizes The number of vertices in each ring, the exterior ring first. The rings are open, i.e. they do
    /// not repeat the first vertex at the end.
    /// A reversed ring keeps its first vertex, so `[v0, v1, ..., vn-1]` becomes `[v0, vn-1, ..., v1]`.
    template <typename Vertex>
    static void wind(std::span<Vertex> vertices, std::span<const std::uint32_t> ringSizes) {
        std::size_t offset = 0;
        bool exterior = true;
        for (const auto ringSize : ringSizes) {
            if (ringSize > vertices.size() - offset) {
                return;
            }
            const auto ring = vertices.subspan(offset, ringSize);
            offset += ringSize;

            const auto sign = areaSign<Vertex>(ring);
            if (exterior ? sign < 0 : sign > 0) {
                std::reverse(ring.begin() + 1, ring.end());
            }
            exterior = false;
        }
    }

    /// The sign of the area of an open ring by the surveyor's formula, as MVT defines winding.
    /// The sum is exact: a term of 32-bit coordinates fits in 64 bits, but their total may not, so it is kept in 128.
    template <typename Vertex>
    static int areaSign(std::span<const Vertex> ring) {
        std::int64_t high = 0;
        std::uint64_t low = 0;
        for (std::size_t i = 0; i < ring.size(); ++i) {
            const auto& a = ring[i];
            const auto& b = ring[i + 1 == ring.size() ? 0 : i + 1];
            const auto term = (static_cast<std::int64_t>(a.x) * b.y) - (static_cast<std::int64_t>(b.x) * a.y);

            // Add the sign-extended term to the 128-bit total
            const auto bits = static_cast<std::uint64_t>(term);
            low += bits;
            high += (term < 0 ? -1 : 0) + (low < bits ? 1 : 0);
        }
        if (high < 0) {
            return -1;
        }
        return (high > 0 || low > 0) ? 1 : 0;
    }
};

} // namespace mlt::encoder
