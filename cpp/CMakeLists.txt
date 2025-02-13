include(FetchContent)

cmake_minimum_required (VERSION 3.12)
set(CMAKE_OSX_DEPLOYMENT_TARGET 15)
project (mlt-cpp LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
add_library(mlt-cpp STATIC)
target_compile_definitions(mlt-cpp PUBLIC PROTOZERO_USE_VIEW=std::string_view)

option(MLT_WITH_JSON "Include JSON support" ON)

set_target_properties(
    mlt-cpp
    PROPERTIES
    INTERFACE_MAPLIBRE_NAME "MapLibre Tile Format"
    INTERFACE_MAPLIBRE_URL "https://github.com/maplibre/maplibre-tile-spec"
    INTERFACE_MAPLIBRE_AUTHOR "MapLibre"
    INTERFACE_MAPLIBRE_LICENSE "${PROJECT_SOURCE_DIR}/LICENSE-APACHE"
)

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
set(BUILD_TESTING OFF CACHE BOOL "Disable tests in protozero" FORCE)
fetchcontent_declare(protozero
    GIT_REPOSITORY https://github.com/mapbox/protozero.git
    GIT_TAG v1.8.0
    EXCLUDE_FROM_ALL
)
FetchContent_MakeAvailable(protozero)
add_library(protozero INTERFACE)
target_include_directories(protozero INTERFACE "${protozero_SOURCE_DIR}/include")

# SIMD Everywhere (needed on ARM)
fetchcontent_declare(simde
  GIT_REPOSITORY https://github.com/simd-everywhere/simde.git
  GIT_TAG v0.8.2
  EXCLUDE_FROM_ALL
)
FetchContent_MakeAvailable(simde)

# FastPFor
set(WITH_TEST OFF CACHE BOOL "Disable tests in FastPFor" FORCE)  # The fastpfor gtest targets conflict with ours
fetchcontent_declare(fastpfor
    GIT_REPOSITORY https://github.com/fast-pack/FastPFor.git
    GIT_TAG v0.3.0
    EXCLUDE_FROM_ALL
)
FetchContent_MakeAvailable(fastpfor)
target_include_directories(mlt-cpp PRIVATE "${simde_SOURCE_DIR}" "${fastpfor_SOURCE_DIR}/headers")
target_compile_options(FastPFOR PRIVATE "-Wno-cast-align")

# FSST
fetchcontent_declare(fsst
    GIT_REPOSITORY https://github.com/cwida/fsst.git
    GIT_BRANCH main # no releases yet
    EXCLUDE_FROM_ALL
)
FetchContent_MakeAvailable(fsst)
target_include_directories(mlt-cpp PRIVATE "${fsst_SOURCE_DIR}")

# json
if(MLT_WITH_JSON)
    message(STATUS "[MLT] Including GeoJSON support")
    fetchcontent_declare(json
        GIT_REPOSITORY https://github.com/nlohmann/json.git
        GIT_TAG v3.11.3
        EXCLUDE_FROM_ALL
    )
    FetchContent_MakeAvailable(json)
    target_include_directories(mlt-cpp PUBLIC "${json_SOURCE_DIR}/include")
    target_compile_definitions(mlt-cpp PUBLIC MLT_WITH_JSON=1)
else()
    message(STATUS "[MLT] No GeoJSON support")
endif(MLT_WITH_JSON)

target_link_libraries(mlt-cpp protozero FastPFOR)

add_subdirectory(${PROJECT_SOURCE_DIR}/test)
add_subdirectory(${PROJECT_SOURCE_DIR}/tool)
