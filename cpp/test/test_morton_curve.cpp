#include <gtest/gtest.h>

#include <mlt/util/morton_curve.hpp>

using MortonCurve = mlt::util::MortonCurve;

// --- Basic Encoding/Decoding Tests ---

TEST(MortonCurve, EncodeOrigin) {
    MortonCurve curve(0, 255);
    auto encoded = curve.encode({0.0f, 0.0f});
    EXPECT_EQ(encoded, 0u);
}

TEST(MortonCurve, EncodeMaxBounds) {
    MortonCurve curve(0, 255);
    // Max bounds should encode to a value with all bits set (within numBits)
    auto encoded = curve.encode({255.0f, 255.0f});
    EXPECT_GT(encoded, 0u);
}

TEST(MortonCurve, EncodeSimpleCoordinates) {
    MortonCurve curve(0, 15);
    // (1, 0) should have only x bit set, interleaved: bit pattern 01 from x, 00 from y = 001
    auto encoded1 = curve.encode({1.0f, 0.0f});
    EXPECT_EQ(encoded1, 1u);

    // (0, 1) should have only y bit set, interleaved: 00 from x, 01 from y = 010
    auto encoded2 = curve.encode({0.0f, 1.0f});
    EXPECT_EQ(encoded2, 2u);

    // (1, 1) should be: 01 from x, 01 from y interleaved = 0011
    auto encoded3 = curve.encode({1.0f, 1.0f});
    EXPECT_EQ(encoded3, 3u);

    // (2, 0) should be: 10 from x (bit 1 set), 00 from y = 0100
    auto encoded4 = curve.encode({2.0f, 0.0f});
    EXPECT_EQ(encoded4, 4u);
}

// --- Roundtrip Tests ---

TEST(MortonCurve, RoundtripSmallRange) {
    MortonCurve curve(0, 15);

    for (int x = 0; x <= 15; ++x) {
        for (int y = 0; y <= 15; ++y) {
            auto encoded = curve.encode({static_cast<float>(x), static_cast<float>(y)});
            auto decoded = curve.decode(encoded);
            EXPECT_EQ(static_cast<int>(decoded.x), x) << "x=" << x << " y=" << y;
            EXPECT_EQ(static_cast<int>(decoded.y), y) << "x=" << x << " y=" << y;
        }
    }
}

TEST(MortonCurve, RoundtripMediumRange) {
    MortonCurve curve(0, 255);

    for (int x = 0; x < 256; x += 16) {
        for (int y = 0; y < 256; y += 16) {
            auto encoded = curve.encode({static_cast<float>(x), static_cast<float>(y)});
            auto decoded = curve.decode(encoded);
            EXPECT_EQ(static_cast<int>(decoded.x), x) << "x=" << x << " y=" << y;
            EXPECT_EQ(static_cast<int>(decoded.y), y) << "x=" << x << " y=" << y;
        }
    }
}

TEST(MortonCurve, RoundtripLargeRange) {
    MortonCurve curve(0, 4095);

    for (int x = 0; x < 4096; x += 512) {
        for (int y = 0; y < 4096; y += 512) {
            auto encoded = curve.encode({static_cast<float>(x), static_cast<float>(y)});
            auto decoded = curve.decode(encoded);
            EXPECT_EQ(static_cast<int>(decoded.x), x) << "x=" << x << " y=" << y;
            EXPECT_EQ(static_cast<int>(decoded.y), y) << "x=" << x << " y=" << y;
        }
    }
}

// --- Negative Coordinate Tests (with coordinate shifting) ---

TEST(MortonCurve, RoundtripNegativeBounds) {
    MortonCurve curve(-128, 127);

    for (int x = -128; x < 128; x += 32) {
        for (int y = -128; y < 128; y += 32) {
            auto encoded = curve.encode({static_cast<float>(x), static_cast<float>(y)});
            auto decoded = curve.decode(encoded);
            EXPECT_EQ(static_cast<int>(decoded.x), x) << "x=" << x << " y=" << y;
            EXPECT_EQ(static_cast<int>(decoded.y), y) << "x=" << x << " y=" << y;
        }
    }
}

TEST(MortonCurve, RoundtripMixedBounds) {
    MortonCurve curve(-256, 255);

    for (int x = -256; x < 256; x += 64) {
        for (int y = -256; y < 256; y += 64) {
            auto encoded = curve.encode({static_cast<float>(x), static_cast<float>(y)});
            auto decoded = curve.decode(encoded);
            EXPECT_EQ(static_cast<int>(decoded.x), x) << "x=" << x << " y=" << y;
            EXPECT_EQ(static_cast<int>(decoded.y), y) << "x=" << x << " y=" << y;
        }
    }
}

// --- Boundary Tests ---

TEST(MortonCurve, BoundaryMin) {
    MortonCurve curve(0, 255);
    auto encoded = curve.encode({0.0f, 0.0f});
    auto decoded = curve.decode(encoded);
    EXPECT_EQ(decoded.x, 0.0f);
    EXPECT_EQ(decoded.y, 0.0f);
}

TEST(MortonCurve, BoundaryMax) {
    MortonCurve curve(0, 255);
    auto encoded = curve.encode({255.0f, 255.0f});
    auto decoded = curve.decode(encoded);
    EXPECT_EQ(static_cast<int>(decoded.x), 255);
    EXPECT_EQ(static_cast<int>(decoded.y), 255);
}

TEST(MortonCurve, BoundaryNegativeMin) {
    MortonCurve curve(-128, 127);
    auto encoded = curve.encode({-128.0f, -128.0f});
    auto decoded = curve.decode(encoded);
    EXPECT_EQ(static_cast<int>(decoded.x), -128);
    EXPECT_EQ(static_cast<int>(decoded.y), -128);
}

TEST(MortonCurve, BoundaryNegativeMax) {
    MortonCurve curve(-128, 127);
    auto encoded = curve.encode({127.0f, 127.0f});
    auto decoded = curve.decode(encoded);
    EXPECT_EQ(static_cast<int>(decoded.x), 127);
    EXPECT_EQ(static_cast<int>(decoded.y), 127);
}

// --- Independent X and Y Tests ---

TEST(MortonCurve, OnlyXVaries) {
    MortonCurve curve(0, 31);

    for (int x = 0; x <= 31; ++x) {
        auto encoded = curve.encode({static_cast<float>(x), 0.0f});
        auto decoded = curve.decode(encoded);
        EXPECT_EQ(static_cast<int>(decoded.x), x);
        EXPECT_EQ(static_cast<int>(decoded.y), 0);
    }
}

TEST(MortonCurve, OnlyYVaries) {
    MortonCurve curve(0, 31);

    for (int y = 0; y <= 31; ++y) {
        auto encoded = curve.encode({0.0f, static_cast<float>(y)});
        auto decoded = curve.decode(encoded);
        EXPECT_EQ(static_cast<int>(decoded.x), 0);
        EXPECT_EQ(static_cast<int>(decoded.y), y);
    }
}

// --- Monotonicity Tests ---

TEST(MortonCurve, IncreasingXIncreasesMortonCode) {
    MortonCurve curve(0, 255);
    std::uint32_t prevCode = 0;

    for (int x = 0; x <= 255; ++x) {
        auto encoded = curve.encode({static_cast<float>(x), 0.0f});
        // Morton code should generally increase (not strictly, but shouldn't jump around)
        EXPECT_GE(encoded, prevCode) << "x=" << x;
        prevCode = encoded;
    }
}

TEST(MortonCurve, IncreasingYIncreasesMortonCode) {
    MortonCurve curve(0, 255);
    std::uint32_t prevCode = 0;

    for (int y = 0; y <= 255; ++y) {
        auto encoded = curve.encode({0.0f, static_cast<float>(y)});
        EXPECT_GE(encoded, prevCode) << "y=" << y;
        prevCode = encoded;
    }
}

// --- Symmetry Tests ---

TEST(MortonCurve, XYSymmetry) {
    MortonCurve curve(0, 255);

    // Test that swapping x and y produces different codes (Morton order is not symmetric like Hilbert)
    auto code1 = curve.encode({10.0f, 20.0f});
    auto code2 = curve.encode({20.0f, 10.0f});
    EXPECT_NE(code1, code2);
}

// --- Static Decode Tests ---

TEST(MortonCurve, StaticDecode) {
    // Test static decode method directly
    auto pt = MortonCurve::decode(0u, 8, 0);
    EXPECT_EQ(pt.x, 0.0f);
    EXPECT_EQ(pt.y, 0.0f);

    // Test with shifted coordinates
    auto pt2 = MortonCurve::decode(1u, 8, 0);
    EXPECT_EQ(static_cast<int>(pt2.x), 1);
    EXPECT_EQ(static_cast<int>(pt2.y), 0);

    auto pt3 = MortonCurve::decode(2u, 8, 0);
    EXPECT_EQ(static_cast<int>(pt3.x), 0);
    EXPECT_EQ(static_cast<int>(pt3.y), 1);
}

// --- Static Component Decode Tests ---

TEST(MortonCurve, StaticComponentDecode) {
    // Test static component decode for x-coordinate (even bits)
    EXPECT_EQ(MortonCurve::decode(0u, 8), 0);  // 0b00000000 -> 0
    EXPECT_EQ(MortonCurve::decode(1u, 8), 1);  // 0b00000001 -> 1 (bit 0 set)
    EXPECT_EQ(MortonCurve::decode(4u, 8), 2);  // 0b00000100 -> 2 (bit 2 set)
    EXPECT_EQ(MortonCurve::decode(16u, 8), 4); // 0b00010000 -> 4 (bit 4 set)
}

// --- Extreme Bit Width Tests ---

TEST(MortonCurve, MinBitWidth) {
    // With a small range, we should have minimal bits
    MortonCurve curve(0, 1);
    EXPECT_EQ(curve.getNumBits(), 1u);

    auto encoded = curve.encode({1.0f, 0.0f});
    auto decoded = curve.decode(encoded);
    EXPECT_EQ(static_cast<int>(decoded.x), 1);
    EXPECT_EQ(static_cast<int>(decoded.y), 0);
}

TEST(MortonCurve, LargeBitWidth) {
    // With a large range, we should have many bits
    MortonCurve curve(0, 65535);
    EXPECT_EQ(curve.getNumBits(), 16u);

    auto encoded = curve.encode({12345.0f, 54321.0f});
    auto decoded = curve.decode(encoded);
    EXPECT_EQ(static_cast<int>(decoded.x), 12345);
    EXPECT_EQ(static_cast<int>(decoded.y), 54321);
}

// --- Validation Tests ---

TEST(MortonCurve, OutOfBoundsThrows) {
    MortonCurve curve(0, 255);

    EXPECT_THROW(curve.encode({256.0f, 0.0f}), std::runtime_error);
    EXPECT_THROW(curve.encode({-1.0f, 0.0f}), std::runtime_error);
    EXPECT_THROW(curve.encode({0.0f, 256.0f}), std::runtime_error);
    EXPECT_THROW(curve.encode({0.0f, -1.0f}), std::runtime_error);
}

TEST(MortonCurve, OutOfBoundsNegativeThrows) {
    MortonCurve curve(-128, 127);

    EXPECT_THROW(curve.encode({128.0f, 0.0f}), std::runtime_error);
    EXPECT_THROW(curve.encode({-129.0f, 0.0f}), std::runtime_error);
}

// --- Getters ---

TEST(MortonCurve, GetNumBits) {
    MortonCurve curve1(0, 255);
    EXPECT_EQ(curve1.getNumBits(), 8u);

    MortonCurve curve2(0, 4095);
    EXPECT_EQ(curve2.getNumBits(), 12u);

    MortonCurve curve3(-128, 127);
    EXPECT_EQ(curve3.getNumBits(), 8u);
}

TEST(MortonCurve, GetCoordinateShift) {
    MortonCurve curve1(0, 255);
    EXPECT_EQ(curve1.getCoordinateShift(), 0);

    MortonCurve curve2(-128, 127);
    EXPECT_EQ(curve2.getCoordinateShift(), 128);

    MortonCurve curve3(-256, 255);
    EXPECT_EQ(curve3.getCoordinateShift(), 256);
}

// --- Consistency Tests ---

TEST(MortonCurve, IdenticalCoordinatesReturnIdenticalCodes) {
    MortonCurve curve(0, 255);

    auto code1 = curve.encode({100.0f, 200.0f});
    auto code2 = curve.encode({100.0f, 200.0f});
    EXPECT_EQ(code1, code2);
}

TEST(MortonCurve, DifferentCoordinatesReturnDifferentCodes) {
    MortonCurve curve(0, 255);

    auto code1 = curve.encode({100.0f, 200.0f});
    auto code2 = curve.encode({100.0f, 201.0f});
    EXPECT_NE(code1, code2);

    auto code3 = curve.encode({101.0f, 200.0f});
    EXPECT_NE(code1, code3);
}

// --- PDF/Reference Values Test ---
// Morton curve (Z-order curve) follows a predictable pattern
// for coordinates in range [0, 2^n):
// (0,0)=0, (1,0)=1, (0,1)=2, (1,1)=3, (2,0)=4, (3,0)=5, (2,1)=6, (3,1)=7, etc.
TEST(MortonCurve, ReferenceValues) {
    MortonCurve curve(0, 255);

    // First 2x2 block (indices 0-3)
    EXPECT_EQ(curve.encode({0.0f, 0.0f}), 0u);
    EXPECT_EQ(curve.encode({1.0f, 0.0f}), 1u);
    EXPECT_EQ(curve.encode({0.0f, 1.0f}), 2u);
    EXPECT_EQ(curve.encode({1.0f, 1.0f}), 3u);

    // Second row (indices 4-7)
    EXPECT_EQ(curve.encode({2.0f, 0.0f}), 4u);
    EXPECT_EQ(curve.encode({3.0f, 0.0f}), 5u);
    EXPECT_EQ(curve.encode({2.0f, 1.0f}), 6u);
    EXPECT_EQ(curve.encode({3.0f, 1.0f}), 7u);

    // Third row (indices 8-11)
    EXPECT_EQ(curve.encode({0.0f, 2.0f}), 8u);
    EXPECT_EQ(curve.encode({1.0f, 2.0f}), 9u);
    EXPECT_EQ(curve.encode({0.0f, 3.0f}), 10u);
    EXPECT_EQ(curve.encode({1.0f, 3.0f}), 11u);
}
