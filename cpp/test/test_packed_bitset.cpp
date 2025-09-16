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
    EXPECT_EQ(nextSetBit({0x01, 0xc0}, 1), 14);
}

TEST(PackedBitset, CountBits) {
    EXPECT_EQ(countSetBits({}), 0);
    EXPECT_EQ(countSetBits({0}), 0);
    EXPECT_EQ(countSetBits({0, 0}), 0);

    EXPECT_EQ(countSetBits({0x01}), 1);
    EXPECT_EQ(countSetBits({0x02}), 1);
    EXPECT_EQ(countSetBits({0x04}), 1);
    EXPECT_EQ(countSetBits({0x08}), 1);
    EXPECT_EQ(countSetBits({0x10}), 1);
    EXPECT_EQ(countSetBits({0x20}), 1);
    EXPECT_EQ(countSetBits({0x40}), 1);
    EXPECT_EQ(countSetBits({0x80}), 1);
    EXPECT_EQ(countSetBits({0x01, 0}), 1);

    EXPECT_EQ(countSetBits({0, 0, 0, 4}), 1);

    EXPECT_EQ(countSetBits({0xff}), 8);
}
