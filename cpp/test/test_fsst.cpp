#include <gtest/gtest.h>

#include <mlt/decode/string.hpp>

#include <string>
#include <vector>

TEST(FSST, DecodeFromJava_decode1) {
    const std::string expected = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";
    const std::vector<std::uint8_t> symbols = {65, 65, 69, 69, 100, 100, 65, 66, 67, 69, 100, 102};
    const std::vector<std::uint32_t> symbolLengths = {2, 2, 2, 1, 1, 1, 1, 1, 1};
    const std::vector<std::uint8_t> javaCompressed = {0, 0, 0, 3, 4, 4, 4, 0, 3, 5, 5, 2, 2, 7, 1,
                                                      1, 1, 8, 8, 8, 1, 1, 0, 0, 3, 2, 2, 5, 5};

    const auto decoded = mlt::decoder::StringDecoder::decodeFSST(
        symbols, symbolLengths, javaCompressed, expected.size());

    EXPECT_EQ(decoded.size(), expected.size());
   //  EXPECT_EQ(0, memcmp(expected.c_str(), decoded.data(), expected.size()));
}

TEST(FSST, DecodeFromJava_decode2){
    const std::string expected = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";
    const std::vector<std::uint8_t> symbols = {65, 65, 69, 69, 100, 100, 65, 66, 67, 69, 100, 102};
    const std::vector<std::uint32_t> symbolLengths = {2, 2, 2, 1, 1, 1, 1, 1, 1};
    const std::vector<std::uint8_t> javaCompressed = {0, 0, 0, 3, 4, 4, 4, 0, 3, 5, 5, 2, 2, 7, 1,
                                                      1, 1, 8, 8, 8, 1, 1, 0, 0, 3, 2, 2, 5, 5};

    // also make sure buffer growth works
    const auto decoded2 = mlt::decoder::StringDecoder::decodeFSST(symbols, symbolLengths, javaCompressed, 0);
    EXPECT_EQ(decoded2.size(), expected.size());
    EXPECT_EQ(0, memcmp(expected.c_str(), decoded2.data(), expected.size()));
}

TEST(FSST, DecodeFromJava_decode3){
    const std::string expected = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";
    const std::vector<std::uint8_t> symbols = {65, 65, 69, 69, 100, 100, 65, 66, 67, 69, 100, 102};
    const std::vector<std::uint32_t> symbolLengths = {2, 2, 2, 1, 1, 1, 1, 1, 1};
    const std::vector<std::uint8_t> javaCompressed = {0, 0, 0, 3, 4, 4, 4, 0, 3, 5, 5, 2, 2, 7, 1,
                                                      1, 1, 8, 8, 8, 1, 1, 0, 0, 3, 2, 2, 5, 5};

    const auto decoded3 = mlt::decoder::StringDecoder::decodeFSST(
        symbols, symbolLengths, javaCompressed, expected.size() / 2);
    EXPECT_EQ(decoded3.size(), expected.size());
    // EXPECT_EQ(0, memcmp(expected.c_str(), decoded3.data(), expected.size() / 2));
}

/*TEST(FSST, DecodeFromJava_decode4){
    const std::string expected = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCCk";
    const std::vector<std::uint8_t> symbols = {65, 65, 69, 69, 100, 100, 65, 66, 67, 69, 100, 102, 255, 107};
    const std::vector<std::uint32_t> symbolLengths = {2, 2, 2, 1, 1, 1, 1, 1, 1};
    const std::vector<std::uint8_t> javaCompressed = {0, 0, 0, 3, 4, 4, 4, 0, 3, 5, 5, 2, 2, 7, 1,
                                                      1, 1, 8, 8, 8, 1, 1, 0, 0, 3, 2, 2, 5, 5};

    const auto decoded4 = mlt::decoder::StringDecoder::decodeFSST(
        symbols, symbolLengths, javaCompressed, expected.size());
    EXPECT_EQ(decoded4.size(), expected.size());
    EXPECT_EQ(0, memcmp(expected.c_str(), decoded4.data(), expected.size()));
}*/
