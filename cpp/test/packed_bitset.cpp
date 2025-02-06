#include <gtest/gtest.h>

#include <mlt/util/packed_bitset.hpp>

using namespace mlt;

TEST(PackedBitset, TestBit) {
    EXPECT_FALSE(testBit({}, 0));
    EXPECT_FALSE(testBit({}, 1));
    EXPECT_FALSE(testBit({}, 1000));

    EXPECT_FALSE(testBit({0xf0}, 0));
    EXPECT_TRUE(testBit({0xf0}, 7));
    EXPECT_FALSE(testBit({0xf0}, 8));

    EXPECT_TRUE(testBit({0xf0, 0x01}, 8));
}

TEST(PackedBitset, NextBit) {
    for (const auto i : {0, 7, 8}) {
        EXPECT_FALSE(nextSetBit({}, i));
        EXPECT_FALSE(nextSetBit({0}, i));
        EXPECT_FALSE(nextSetBit({0, 0}, i));
    }

    EXPECT_EQ(nextSetBit({0xaa}, 0), 1);
    EXPECT_EQ(nextSetBit({0xaa}, 1), 1);
    EXPECT_EQ(nextSetBit({0xaa}, 2), 3);
    EXPECT_EQ(nextSetBit({0xaa}, 3), 3);
    EXPECT_EQ(nextSetBit({0xaa}, 4), 5);
    EXPECT_EQ(nextSetBit({0xaa}, 5), 5);
    EXPECT_EQ(nextSetBit({0xaa}, 6), 7);
    EXPECT_EQ(nextSetBit({0xaa}, 7), 7);
    EXPECT_FALSE(nextSetBit({0xaa}, 8));
    EXPECT_EQ(nextSetBit({0xaa, 0xaa}, 8), 9);

    EXPECT_EQ(nextSetBit({0x0c, 0}, 0), 2);
    EXPECT_EQ(nextSetBit({0x0c, 0}, 3), 3);
    EXPECT_FALSE(nextSetBit({0x0c, 0}, 4));
}
