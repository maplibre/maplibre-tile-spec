cmake_minimum_required(VERSION 3.12)
set(CMAKE_OSX_DEPLOYMENT_TARGET 15)
project(mlt-cpp LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
add_library(mlt-cpp STATIC)

option(MLT_WITH_JSON "Include JSON support" ON)
option(MLT_WITH_PROTOZERO "Include Protozero decoding of tileset metadata" ON)

set_target_properties(
    mlt-cpp
    PROPERTIES
    INTERFACE_MAPLIBRE_NAME "MapLibre Tile Format"
    INTERFACE_MAPLIBRE_URL "https://github.com/maplibre/maplibre-tile-spec"
    INTERFACE_MAPLIBRE_AUTHOR "MapLibre"
    INTERFACE_MAPLIBRE_LICENSE "${PROJECT_SOURCE_DIR}/../LICENSE-APACHE"
)

target_include_directories(mlt-cpp
    PUBLIC ${PROJECT_SOURCE_DIR}/include
    PRIVATE ${PROJECT_SOURCE_DIR}/src
)

list(APPEND MLT_INCLUDE_FILES
    ${PROJECT_SOURCE_DIR}/include/mlt/common.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/decoder.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/feature.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/geometry.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/layer.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/polyfill.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/projection.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/properties.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/tile.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/metadata/stream.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/metadata/tileset.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/metadata/tileset_protozero.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/buffer_stream.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/packed_bitset.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/noncopyable.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/stl.hpp
)

list(APPEND MLT_SRC_FILES
    ${PROJECT_SOURCE_DIR}/src/mlt/decoder.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/geometry.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/int.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/int.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/property.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/string.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/feature.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/metadata/stream.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/morton.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/raw.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/rle.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/rle.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/varint.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/vectorized.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/zigzag.hpp
)

if(MLT_WITH_JSON)
    list(APPEND MLT_INCLUDE_FILES ${PROJECT_SOURCE_DIR}/include/mlt/geojson.hpp)
endif(MLT_WITH_JSON)

target_sources(mlt-cpp PRIVATE
    ${MLT_INCLUDE_FILES}
    ${MLT_SRC_FILES}
)

# SIMD Everywhere (needed on ARM)
target_include_directories(mlt-cpp PRIVATE "${PROJECT_SOURCE_DIR}/vendor/simde")

# FastPFor
set(WITH_TEST OFF CACHE BOOL "Disable tests in FastPFor" FORCE) # The fastpfor gtest targets conflict with ours

if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.25")
    add_subdirectory(${PROJECT_SOURCE_DIR}/vendor/fastpfor SYSTEM)
else()
    add_subdirectory(${PROJECT_SOURCE_DIR}/vendor/fastpfor)
    target_compile_options(FastPFOR PRIVATE "-Wno-cast-align")
endif()

target_link_libraries(mlt-cpp FastPFOR)
target_include_directories(mlt-cpp PUBLIC "${PROJECT_SOURCE_DIR}/vendor/fastpfor/headers")
list(APPEND MLT_EXPORT_TARGETS FastPFOR)

# FSST
if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.25")
    add_subdirectory(${PROJECT_SOURCE_DIR}/vendor/fsst SYSTEM)
else()
    add_subdirectory(${PROJECT_SOURCE_DIR}/vendor/fsst)
endif()

target_include_directories(mlt-cpp PUBLIC "${PROJECT_SOURCE_DIR}/vendor/fsst")

# Protozero
if(MLT_WITH_PROTOZERO)
    message(STATUS "[MLT] Including Protozero")
    set(DOXYGEN_FOUND OFF CACHE BOOL "Disable doxygen in protozero" FORCE)
    set(BUILD_TESTING OFF CACHE BOOL "Disable tests in protozero" FORCE)

    if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.25")
        add_subdirectory(${PROJECT_SOURCE_DIR}/vendor/protozero SYSTEM)
    else()
        set(WERROR OFF CACHE BOOL "Disable warnings in protozero" FORCE)
        add_subdirectory(${PROJECT_SOURCE_DIR}/vendor/protozero)
    endif()

    target_include_directories(mlt-cpp PUBLIC "${PROJECT_SOURCE_DIR}/vendor/protozero/include")
    target_compile_definitions(mlt-cpp PUBLIC PROTOZERO_USE_VIEW=std::string_view MLT_WITH_PROTOZERO=1)
else()
    message(STATUS "[MLT] No Protozero")
endif(MLT_WITH_PROTOZERO)

# json
if(MLT_WITH_JSON)
    message(STATUS "[MLT] Including GeoJSON support")

    if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.25")
        add_subdirectory(${PROJECT_SOURCE_DIR}/vendor/json SYSTEM)
    else()
        add_subdirectory(${PROJECT_SOURCE_DIR}/vendor/json)
    endif()

    target_include_directories(mlt-cpp PUBLIC "${PROJECT_SOURCE_DIR}/vendor/json/include")
    target_compile_definitions(mlt-cpp PUBLIC MLT_WITH_JSON=1)
else()
    message(STATUS "[MLT] No GeoJSON support")
endif(MLT_WITH_JSON)

if(MLT_WITH_PROTOZERO)
    add_subdirectory(${PROJECT_SOURCE_DIR}/test EXCLUDE_FROM_ALL)
    add_subdirectory(${PROJECT_SOURCE_DIR}/tool EXCLUDE_FROM_ALL)
endif()

export(TARGETS ${MLT_EXPORT_TARGETS} FILE MLTTargets.cmake)
