#include <gtest/gtest.h>

#include <mlt/util/hilbert_curve.hpp>

#include <algorithm>
#include <array>
#include <cstdint>
#include <cstdlib>
#include <utility>

using HilbertCurve = mlt::util::HilbertCurve;

namespace {

void assertRoundTrip(const HilbertCurve& curve, int x, int y) {
    const auto index = curve.encode({static_cast<float>(x), static_cast<float>(y)});
    const auto decoded = curve.decode(index);
    EXPECT_EQ(static_cast<int>(decoded.x), x) << "x=" << x << " y=" << y;
    EXPECT_EQ(static_cast<int>(decoded.y), y) << "x=" << x << " y=" << y;
}

void assertRoundTripForLevel(int level) {
    const auto maxCoordinate = (1 << level) - 1;
    const HilbertCurve curve(0, maxCoordinate);

    assertRoundTrip(curve, 0, 0);
    assertRoundTrip(curve, maxCoordinate, 0);
    assertRoundTrip(curve, 0, maxCoordinate);
    assertRoundTrip(curve, maxCoordinate, maxCoordinate);
    assertRoundTrip(curve, maxCoordinate / 2, maxCoordinate / 2);

    const int step = std::max(1, maxCoordinate / 7);
    for (int y = 0; y <= maxCoordinate; y += step) {
        for (int x = 0; x <= maxCoordinate; x += step) {
            assertRoundTrip(curve, x, y);
        }
    }
}
} // namespace

class HilbertCurveRoundTripForLevelTest : public ::testing::TestWithParam<int> {};

TEST(HilbertCurve, Level1HasExpectedStandardOrder) {
    const HilbertCurve curve(0, 1);

    constexpr std::array<std::pair<int, int>, 4> expected = {
        std::pair<int, int>{0, 0},
        {0, 1},
        {1, 1},
        {1, 0},
    };

    for (std::uint32_t index = 0; index < expected.size(); ++index) {
        const auto decoded = curve.decode(index);
        EXPECT_EQ(static_cast<int>(decoded.x), expected[index].first) << "index=" << index;
        EXPECT_EQ(static_cast<int>(decoded.y), expected[index].second) << "index=" << index;
    }
}

TEST_P(HilbertCurveRoundTripForLevelTest, EncodeDecodeRoundTripsRepresentativeLevel) {
    assertRoundTripForLevel(GetParam());
}

INSTANTIATE_TEST_SUITE_P(RepresentativeLevels,
                         HilbertCurveRoundTripForLevelTest,
                         ::testing::ValuesIn({1, 2, 5, 10, 16}));

TEST(HilbertCurve, ConsecutiveIndicesAreAxisAdjacentAtLevel6) {
    constexpr int level = 6;
    const auto maxCoordinate = (1 << level) - 1;
    const HilbertCurve curve(0, maxCoordinate);

    const std::uint32_t maxIndexExclusive = 1u << (2 * level);
    for (std::uint32_t index = 0; index + 1 < maxIndexExclusive; ++index) {
        const auto a = curve.decode(index);
        const auto b = curve.decode(index + 1);

        const int manhattan = std::abs(static_cast<int>(a.x) - static_cast<int>(b.x)) +
                              std::abs(static_cast<int>(a.y) - static_cast<int>(b.y));
        EXPECT_EQ(manhattan, 1) << "index=" << index;
    }
}
