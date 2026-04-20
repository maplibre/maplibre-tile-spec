#include <gtest/gtest.h>

#include <mlt/metadata/type_map.hpp>

using namespace mlt::metadata;
using namespace mlt::metadata::tileset;

class Tag0x01TypeMapTest : public ::testing::Test {
protected:
    using Tag0x01 = type_map::Tag0x01;
};

// --- Scalar Type Roundtrip Tests ---

TEST_F(Tag0x01TypeMapTest, BooleanNonNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::BOOLEAN, {}, {}, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 10u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->hasScalarType());
    EXPECT_FALSE(decoded->nullable);
    EXPECT_TRUE(decoded->getScalarType().hasPhysicalType());
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::BOOLEAN);
}

TEST_F(Tag0x01TypeMapTest, BooleanNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::BOOLEAN, {}, {}, {}, true, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 11u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->hasScalarType());
    EXPECT_TRUE(decoded->nullable);
    EXPECT_TRUE(decoded->getScalarType().hasPhysicalType());
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::BOOLEAN);
}

TEST_F(Tag0x01TypeMapTest, Int8NonNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::INT_8, {}, {}, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 12u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->hasScalarType());
    EXPECT_FALSE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::INT_8);
}

TEST_F(Tag0x01TypeMapTest, Int8NullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::INT_8, {}, {}, {}, true, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 13u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::INT_8);
}

TEST_F(Tag0x01TypeMapTest, UInt8NonNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::UINT_8, {}, {}, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 14u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_FALSE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::UINT_8);
}

TEST_F(Tag0x01TypeMapTest, UInt8NullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::UINT_8, {}, {}, {}, true, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 15u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::UINT_8);
}

TEST_F(Tag0x01TypeMapTest, Int32NonNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::INT_32, {}, {}, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 16u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_FALSE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::INT_32);
}

TEST_F(Tag0x01TypeMapTest, Int32NullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::INT_32, {}, {}, {}, true, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 17u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::INT_32);
}

TEST_F(Tag0x01TypeMapTest, UInt32NonNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::UINT_32, {}, {}, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 18u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_FALSE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::UINT_32);
}

TEST_F(Tag0x01TypeMapTest, UInt32NullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::UINT_32, {}, {}, {}, true, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 19u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::UINT_32);
}

TEST_F(Tag0x01TypeMapTest, Int64NonNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::INT_64, {}, {}, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 20u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_FALSE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::INT_64);
}

TEST_F(Tag0x01TypeMapTest, Int64NullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::INT_64, {}, {}, {}, true, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 21u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::INT_64);
}

TEST_F(Tag0x01TypeMapTest, UInt64NonNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::UINT_64, {}, {}, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 22u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_FALSE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::UINT_64);
}

TEST_F(Tag0x01TypeMapTest, UInt64NullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::UINT_64, {}, {}, {}, true, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 23u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::UINT_64);
}

TEST_F(Tag0x01TypeMapTest, FloatNonNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::FLOAT, {}, {}, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 24u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_FALSE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::FLOAT);
}

TEST_F(Tag0x01TypeMapTest, FloatNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::FLOAT, {}, {}, {}, true, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 25u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::FLOAT);
}

TEST_F(Tag0x01TypeMapTest, DoubleNonNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::DOUBLE, {}, {}, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 26u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_FALSE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::DOUBLE);
}

TEST_F(Tag0x01TypeMapTest, DoubleNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::DOUBLE, {}, {}, {}, true, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 27u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::DOUBLE);
}

TEST_F(Tag0x01TypeMapTest, StringNonNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::STRING, {}, {}, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 28u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_FALSE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::STRING);
}

TEST_F(Tag0x01TypeMapTest, StringNullableRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType(ScalarType::STRING, {}, {}, {}, true, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 29u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->nullable);
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::STRING);
}

// --- ID Type Roundtrip Tests ---

TEST_F(Tag0x01TypeMapTest, IDNonNullableNonLongRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType({}, LogicalScalarType::ID, {}, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 0u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->hasScalarType());
    EXPECT_FALSE(decoded->nullable);
    EXPECT_TRUE(decoded->getScalarType().isID());
    EXPECT_FALSE(decoded->getScalarType().hasLongID);
}

TEST_F(Tag0x01TypeMapTest, IDNullableNonLongRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType({}, LogicalScalarType::ID, {}, {}, true, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 1u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->nullable);
    EXPECT_TRUE(decoded->getScalarType().isID());
    EXPECT_FALSE(decoded->getScalarType().hasLongID);
}

TEST_F(Tag0x01TypeMapTest, IDNonNullableLongRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType({}, LogicalScalarType::ID, {}, {}, false, false, true);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 2u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_FALSE(decoded->nullable);
    EXPECT_TRUE(decoded->getScalarType().isID());
    EXPECT_TRUE(decoded->getScalarType().hasLongID);
}

TEST_F(Tag0x01TypeMapTest, IDNullableLongRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType({}, LogicalScalarType::ID, {}, {}, true, false, true);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 3u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->nullable);
    EXPECT_TRUE(decoded->getScalarType().isID());
    EXPECT_TRUE(decoded->getScalarType().hasLongID);
}

// --- Complex Type Roundtrip Tests ---

TEST_F(Tag0x01TypeMapTest, GeometryRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType({}, {}, ComplexType::GEOMETRY, {}, false, false, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 4u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->hasComplexType());
    EXPECT_FALSE(decoded->nullable);
    EXPECT_TRUE(decoded->getComplexType().isGeometry());
    EXPECT_FALSE(decoded->getComplexType().hasChildren());
}

TEST_F(Tag0x01TypeMapTest, StructRoundtrip) {
    auto encoded = Tag0x01::encodeColumnType({}, {}, ComplexType::STRUCT, {}, false, true, false);
    ASSERT_TRUE(encoded);
    EXPECT_EQ(*encoded, 30u);

    auto decoded = Tag0x01::decodeColumnType(*encoded);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->hasComplexType());
    EXPECT_FALSE(decoded->nullable);
    EXPECT_TRUE(decoded->getComplexType().isStruct());
}

// --- Invalid Encode Cases (should return nullopt) ---

TEST_F(Tag0x01TypeMapTest, EncodeScalarWithChildrenReturnsNullopt) {
    // Scalar types should not have children
    auto encoded = Tag0x01::encodeColumnType(ScalarType::INT_32, {}, {}, {}, false, true, false);
    EXPECT_FALSE(encoded);
}

TEST_F(Tag0x01TypeMapTest, EncodeIDWithChildrenSucceeds) {
    // ID types encode successfully even with hasChildren=true
    // (the children flag is not checked for logical types)
    auto encoded = Tag0x01::encodeColumnType({}, LogicalScalarType::ID, {}, {}, false, true, false);
    EXPECT_TRUE(encoded);
    EXPECT_EQ(*encoded, 0u);
}

TEST_F(Tag0x01TypeMapTest, EncodeGeometryWithChildrenReturnsNullopt) {
    // Geometry should not have children
    auto encoded = Tag0x01::encodeColumnType({}, {}, ComplexType::GEOMETRY, {}, false, true, false);
    EXPECT_FALSE(encoded);
}

TEST_F(Tag0x01TypeMapTest, EncodeGeometryNullableReturnsNullopt) {
    // Geometry must not be nullable
    auto encoded = Tag0x01::encodeColumnType({}, {}, ComplexType::GEOMETRY, {}, true, false, false);
    EXPECT_FALSE(encoded);
}

TEST_F(Tag0x01TypeMapTest, EncodeStructNullableReturnsNullopt) {
    // Struct must not be nullable
    auto encoded = Tag0x01::encodeColumnType({}, {}, ComplexType::STRUCT, {}, true, true, false);
    EXPECT_FALSE(encoded);
}

TEST_F(Tag0x01TypeMapTest, EncodeStructWithoutChildrenReturnsNullopt) {
    // Struct must have children
    auto encoded = Tag0x01::encodeColumnType({}, {}, ComplexType::STRUCT, {}, false, false, false);
    EXPECT_FALSE(encoded);
}

TEST_F(Tag0x01TypeMapTest, EncodePreferPhysicalScalarOverLogical) {
    // When both physical scalar and logical scalar are set, physical is preferred
    // (due to if-else chain in encodeColumnType)
    auto encoded = Tag0x01::encodeColumnType(ScalarType::INT_32, LogicalScalarType::ID, {}, {}, false, false, false);
    EXPECT_TRUE(encoded);
    // Should encode as INT_32, not ID
    EXPECT_EQ(*encoded, 16u);
}

TEST_F(Tag0x01TypeMapTest, EncodeNothingReturnsNullopt) {
    // At least one type must be set
    auto encoded = Tag0x01::encodeColumnType({}, {}, {}, {}, false, false, false);
    EXPECT_FALSE(encoded);
}

TEST_F(Tag0x01TypeMapTest, EncodeInvalidPhysicalScalarTypeThrows) {
    // Drives Tag0x01::mapScalarType(ScalarType, bool) default throw path.
    EXPECT_THROW((void)Tag0x01::encodeColumnType(static_cast<ScalarType>(999), {}, {}, {}, false, false, false),
                 std::runtime_error);
}

// --- Invalid Decode Cases ---

TEST_F(Tag0x01TypeMapTest, DecodeInvalidRangeReturnsNullopt) {
    // Test codes that are not in any valid range
    EXPECT_FALSE(Tag0x01::decodeColumnType(5u));
    EXPECT_FALSE(Tag0x01::decodeColumnType(6u));
    EXPECT_FALSE(Tag0x01::decodeColumnType(7u));
    EXPECT_FALSE(Tag0x01::decodeColumnType(8u));
    EXPECT_FALSE(Tag0x01::decodeColumnType(9u));
}

TEST_F(Tag0x01TypeMapTest, DecodeInvalidHighRangeReturnsNullopt) {
    // Test codes higher than max valid
    EXPECT_FALSE(Tag0x01::decodeColumnType(31u));
    EXPECT_FALSE(Tag0x01::decodeColumnType(100u));
    EXPECT_FALSE(Tag0x01::decodeColumnType(1000u));
}

TEST_F(Tag0x01TypeMapTest, DecodeAllValidScalarCodes) {
    // Verify all scalar codes decode successfully
    for (uint32_t code = 10; code <= 29; ++code) {
        auto decoded = Tag0x01::decodeColumnType(code);
        EXPECT_TRUE(decoded) << "Code " << code << " should be valid";
        EXPECT_TRUE(decoded->hasScalarType()) << "Code " << code << " should be scalar type";
    }
}

TEST_F(Tag0x01TypeMapTest, DecodeAllValidIDCodes) {
    // Verify all ID codes (0-3) decode successfully
    for (uint32_t code = 0; code <= 3; ++code) {
        auto decoded = Tag0x01::decodeColumnType(code);
        EXPECT_TRUE(decoded) << "Code " << code << " should be valid";
        EXPECT_TRUE(decoded->hasScalarType()) << "Code " << code << " should be scalar type";
        EXPECT_TRUE(decoded->getScalarType().isID()) << "Code " << code << " should be ID type";
    }
}

// --- Helper Method Tests ---

TEST_F(Tag0x01TypeMapTest, ColumnTypeHasNameBelowThreshold) {
    EXPECT_FALSE(Tag0x01::columnTypeHasName(0u));
    EXPECT_FALSE(Tag0x01::columnTypeHasName(1u));
    EXPECT_FALSE(Tag0x01::columnTypeHasName(2u));
    EXPECT_FALSE(Tag0x01::columnTypeHasName(3u));
    EXPECT_FALSE(Tag0x01::columnTypeHasName(4u));
    EXPECT_FALSE(Tag0x01::columnTypeHasName(9u));
}

TEST_F(Tag0x01TypeMapTest, ColumnTypeHasNameAtAndAboveThreshold) {
    EXPECT_TRUE(Tag0x01::columnTypeHasName(10u));
    EXPECT_TRUE(Tag0x01::columnTypeHasName(15u));
    EXPECT_TRUE(Tag0x01::columnTypeHasName(20u));
    EXPECT_TRUE(Tag0x01::columnTypeHasName(29u));
    EXPECT_TRUE(Tag0x01::columnTypeHasName(30u));
    EXPECT_TRUE(Tag0x01::columnTypeHasName(100u));
}

TEST_F(Tag0x01TypeMapTest, ColumnTypeHasChildrenOnlyForStruct) {
    for (uint32_t code = 0; code <= 31; ++code) {
        if (code == 30) {
            EXPECT_TRUE(Tag0x01::columnTypeHasChildren(code)) << "Code 30 (STRUCT) should have children";
        } else {
            EXPECT_FALSE(Tag0x01::columnTypeHasChildren(code)) << "Code " << code << " should not have children";
        }
    }
}

// --- hasStreamCount Tests ---

TEST_F(Tag0x01TypeMapTest, HasStreamCountForString) {
    Column stringCol{.name = "test",
                     .nullable = false,
                     .columnScope = ColumnScope::FEATURE,
                     .type = ScalarColumn{.type = ScalarType::STRING, .hasLongID = false}};
    EXPECT_TRUE(Tag0x01::hasStreamCount(stringCol));
}

TEST_F(Tag0x01TypeMapTest, NoStreamCountForNumericTypes) {
    std::vector<ScalarType> numericTypes = {ScalarType::BOOLEAN,
                                            ScalarType::INT_8,
                                            ScalarType::UINT_8,
                                            ScalarType::INT_32,
                                            ScalarType::UINT_32,
                                            ScalarType::INT_64,
                                            ScalarType::UINT_64,
                                            ScalarType::FLOAT,
                                            ScalarType::DOUBLE};

    for (const auto& type : numericTypes) {
        Column col{.name = "test",
                   .nullable = false,
                   .columnScope = ColumnScope::FEATURE,
                   .type = ScalarColumn{.type = type, .hasLongID = false}};
        EXPECT_FALSE(Tag0x01::hasStreamCount(col)) << "Type should not have stream count";
    }
}

TEST_F(Tag0x01TypeMapTest, NoStreamCountForIDType) {
    Column idCol{.name = "test",
                 .nullable = false,
                 .columnScope = ColumnScope::FEATURE,
                 .type = ScalarColumn{.type = LogicalScalarType::ID, .hasLongID = false}};
    EXPECT_FALSE(Tag0x01::hasStreamCount(idCol));
}

TEST_F(Tag0x01TypeMapTest, HasStreamCountForGeometry) {
    Column geomCol{.name = "test",
                   .nullable = false,
                   .columnScope = ColumnScope::FEATURE,
                   .type = ComplexColumn{.type = ComplexType::GEOMETRY, .children = {}}};
    EXPECT_TRUE(Tag0x01::hasStreamCount(geomCol));
}

TEST_F(Tag0x01TypeMapTest, HasStreamCountForStruct) {
    Column structCol{.name = "test",
                     .nullable = false,
                     .columnScope = ColumnScope::FEATURE,
                     .type = ComplexColumn{.type = ComplexType::STRUCT, .children = {}}};
    EXPECT_TRUE(Tag0x01::hasStreamCount(structCol));
}

TEST_F(Tag0x01TypeMapTest, HasStreamCountInvalidComplexPhysicalTypeThrows) {
    // Drives switch default: break in complex physical type handling, then fallback throw.
    Column invalidComplexCol{.name = "test",
                             .nullable = false,
                             .columnScope = ColumnScope::FEATURE,
                             .type = ComplexColumn{.type = static_cast<ComplexType>(999), .children = {}}};
    EXPECT_THROW((void)Tag0x01::hasStreamCount(invalidComplexCol), std::runtime_error);
}

TEST_F(Tag0x01TypeMapTest, HasStreamCountLogicalComplexTypeThrows) {
    // Drives fallback throw when complex column is not a physical complex type.
    Column logicalComplexCol{.name = "test",
                             .nullable = false,
                             .columnScope = ColumnScope::FEATURE,
                             .type = ComplexColumn{.type = static_cast<LogicalComplexType>(0), .children = {}}};
    EXPECT_THROW((void)Tag0x01::hasStreamCount(logicalComplexCol), std::runtime_error);
}

// --- Comprehensive Nullability Tests ---

TEST_F(Tag0x01TypeMapTest, AllScalarTypesRespectNullability) {
    std::vector<ScalarType> scalarTypes = {ScalarType::BOOLEAN,
                                           ScalarType::INT_8,
                                           ScalarType::UINT_8,
                                           ScalarType::INT_32,
                                           ScalarType::UINT_32,
                                           ScalarType::INT_64,
                                           ScalarType::UINT_64,
                                           ScalarType::FLOAT,
                                           ScalarType::DOUBLE,
                                           ScalarType::STRING};

    for (const auto& scalarType : scalarTypes) {
        // Test non-nullable
        auto encNonNull = Tag0x01::encodeColumnType(scalarType, {}, {}, {}, false, false, false);
        ASSERT_TRUE(encNonNull) << "Should encode non-nullable " << static_cast<int>(scalarType);
        auto decNonNull = Tag0x01::decodeColumnType(*encNonNull);
        ASSERT_TRUE(decNonNull);
        EXPECT_FALSE(decNonNull->nullable);

        // Test nullable
        auto encNull = Tag0x01::encodeColumnType(scalarType, {}, {}, {}, true, false, false);
        ASSERT_TRUE(encNull) << "Should encode nullable " << static_cast<int>(scalarType);
        auto decNull = Tag0x01::decodeColumnType(*encNull);
        ASSERT_TRUE(decNull);
        EXPECT_TRUE(decNull->nullable);

        // Verify they encode to different codes
        EXPECT_NE(*encNonNull, *encNull);
    }
}

// --- Edge Cases ---

TEST_F(Tag0x01TypeMapTest, DecodeCodeZeroIsIDType) {
    auto decoded = Tag0x01::decodeColumnType(0u);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->getScalarType().isID());
}

TEST_F(Tag0x01TypeMapTest, DecodeCodeFourIsGeometry) {
    auto decoded = Tag0x01::decodeColumnType(4u);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->getComplexType().isGeometry());
}

TEST_F(Tag0x01TypeMapTest, DecodeCodeThirtyIsStruct) {
    auto decoded = Tag0x01::decodeColumnType(30u);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->getComplexType().isStruct());
}

// --- Type Consistency ---

TEST_F(Tag0x01TypeMapTest, IDTypeHasCorrectLogicalType) {
    auto decoded = Tag0x01::decodeColumnType(0u);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->getScalarType().hasLogicalType());
    EXPECT_EQ(decoded->getScalarType().getLogicalType(), LogicalScalarType::ID);
}

TEST_F(Tag0x01TypeMapTest, ScalarTypeHasCorrectPhysicalType) {
    auto decoded = Tag0x01::decodeColumnType(10u);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->getScalarType().hasPhysicalType());
    EXPECT_EQ(decoded->getScalarType().getPhysicalType(), ScalarType::BOOLEAN);
}

TEST_F(Tag0x01TypeMapTest, GeometryTypeHasCorrectPhysicalType) {
    auto decoded = Tag0x01::decodeColumnType(4u);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->getComplexType().hasPhysicalType());
    EXPECT_EQ(decoded->getComplexType().getPhysicalType(), ComplexType::GEOMETRY);
}

TEST_F(Tag0x01TypeMapTest, StructTypeHasCorrectPhysicalType) {
    auto decoded = Tag0x01::decodeColumnType(30u);
    ASSERT_TRUE(decoded);
    EXPECT_TRUE(decoded->getComplexType().hasPhysicalType());
    EXPECT_EQ(decoded->getComplexType().getPhysicalType(), ComplexType::STRUCT);
}
