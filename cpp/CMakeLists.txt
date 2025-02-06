include(FetchContent)

cmake_minimum_required (VERSION 3.12)
set(CMAKE_OSX_DEPLOYMENT_TARGET 15)
project (mlt-cpp LANGUAGES CXX)

add_library(mlt-cpp STATIC)
set_property(TARGET mlt-cpp PROPERTY CXX_STANDARD 20)
target_compile_definitions(mlt-cpp PUBLIC PROTOZERO_USE_VIEW=std::string_view)

option(MLT_WITH_JSON "Include JSON support" ON)

target_include_directories(mlt-cpp
    PUBLIC ${PROJECT_SOURCE_DIR}/include
    PRIVATE ${PROJECT_SOURCE_DIR}/src
)

list(APPEND INCLUDE_FILES
    ${PROJECT_SOURCE_DIR}/include/mlt/common.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/decoder.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/feature.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/geometry.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/geojson.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/layer.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/polyfill.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/projection.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/properties.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/tile.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/metadata/stream.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/metadata/tileset.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/buffer_stream.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/packed_bitset.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/noncopyable.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/stl.hpp
)

list(APPEND SRC_FILES
    ${PROJECT_SOURCE_DIR}/src/mlt/decoder.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/geometry.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/int.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/int.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/property.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/string.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/feature.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/metadata/stream.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/metadata/tileset.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/morton.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/raw.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/rle.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/rle.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/varint.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/vectorized.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/zigzag.hpp
)

target_sources(mlt-cpp PRIVATE
    ${INCLUDE_FILES}
    ${SRC_FILES}
)

# Protozero
set(WERROR OFF)
set(DOXYGEN_FOUND OFF)
fetchcontent_declare(protozero
    GIT_REPOSITORY https://github.com/mapbox/protozero.git
    GIT_TAG v1.7.1
)
FetchContent_MakeAvailable(protozero)
target_include_directories(mlt-cpp PRIVATE "${protozero_SOURCE_DIR}/include")

# FastPFor
set(WITH_TEST OFF)  # The fastpfor gtest targets conflict with ours
fetchcontent_declare(fastpfor
    GIT_REPOSITORY https://github.com/fast-pack/FastPFor.git
    GIT_TAG v0.2.0
)
FetchContent_MakeAvailable(fastpfor)
target_include_directories(mlt-cpp PRIVATE "${fastpfor_SOURCE_DIR}/headers")
target_compile_options(FastPFOR PRIVATE "-Wno-cast-align")

# json
if(MLT_WITH_JSON)
    message(STATUS "[MLT] Including GeoJSON support")
    fetchcontent_declare(json
        GIT_REPOSITORY https://github.com/nlohmann/json.git
        GIT_TAG v3.11.3
    )
    FetchContent_MakeAvailable(json)
    target_include_directories(mlt-cpp PUBLIC "${json_SOURCE_DIR}/include")
    target_compile_definitions(mlt-cpp PUBLIC MLT_WITH_JSON=1)
else()
    message(STATUS "[MLT] No GeoJSON support")
endif(MLT_WITH_JSON)

add_subdirectory(${PROJECT_SOURCE_DIR}/test)
