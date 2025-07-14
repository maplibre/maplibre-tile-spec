cmake_minimum_required(VERSION 3.12)
project(mlt-cpp LANGUAGES CXX)

list(APPEND CMAKE_MODULE_PATH ${CMAKE_CURRENT_LIST_DIR}/cmake)
include(AddCXXCompilerFlag)

# Use the same deployment target as the parent project, if set.
if (NOT MLT_OSX_DEPLOYMENT_TARGET)
    set(MLT_OSX_DEPLOYMENT_TARGET 16)
endif()
set(CMAKE_OSX_DEPLOYMENT_TARGET ${MLT_OSX_DEPLOYMENT_TARGET})

set(CMAKE_CXX_STANDARD 20)
add_library(mlt-cpp STATIC)

option(MLT_WITH_JSON "Include JSON support" ON)
option(MLT_WITH_FASTPFOR "Include FastPFor support" ON)
option(MLT_WITH_PROTOZERO "Include Protozero decoding of tileset metadata" ON)

set_target_properties(
    mlt-cpp
    PROPERTIES
    INTERFACE_MAPLIBRE_NAME "MapLibre Tile Format"
    INTERFACE_MAPLIBRE_URL "https://github.com/maplibre/maplibre-tile-spec"
    INTERFACE_MAPLIBRE_AUTHOR "MapLibre"
    INTERFACE_MAPLIBRE_LICENSE "${PROJECT_SOURCE_DIR}/../LICENSE-APACHE"
)

target_compile_options(mlt-cpp
    PRIVATE
        -Werror
        -Wno-trigraphs
        -Wno-missing-field-initializers
        -Wno-return-type
        -Wno-non-virtual-dtor
        -Wno-overloaded-virtual
        -Wno-exit-time-destructors
        -Wno-missing-braces
        -Wparentheses
        -Wswitch
        -Wno-unused-function
        -Wno-unused-label
        -Wno-unused-parameter
        -Wno-unused-variable
        -Wunused-value
        -Wno-empty-body
        -Wno-uninitialized
        -Wno-unknown-pragmas
        -Wno-shadow
        -Wno-four-char-constants
        -Wno-conversion
        -Wno-constant-conversion
        -Wno-bool-conversion
        -Wno-enum-conversion
        -Wno-float-conversion
        -Wno-non-literal-null-conversion
        -Wno-objc-literal-conversion
        -Wno-newline-eof
        -Wno-c++11-extensions
        -Wno-implicit-fallthrough
        -fstrict-aliasing
        -Wdeprecated-declarations
        -Winvalid-offsetof
        -Wno-sign-conversion
        -Wno-infinite-recursion
        -Wno-move
        -Wno-comma
        -Wno-block-capture-autoreleasing
        -Wno-range-loop-analysis
        -Wno-semicolon-before-method-body
)

add_cxx_compiler_flag(-Wshorten-64-to-32)

target_include_directories(mlt-cpp
    PUBLIC ${PROJECT_SOURCE_DIR}/include
    PRIVATE ${PROJECT_SOURCE_DIR}/src
)

list(APPEND MLT_INCLUDE_FILES
    ${PROJECT_SOURCE_DIR}/include/mlt/common.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/decoder.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/feature.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/geometry.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/geometry_vector.hpp
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
    ${PROJECT_SOURCE_DIR}/src/mlt/geometry_vector.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/layer.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/metadata/stream.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/properties.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/morton_curve.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/raw.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/rle.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/rle.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/space_filling_curve.hpp
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
target_include_directories(mlt-cpp PRIVATE "${PROJECT_SOURCE_DIR}/../vendor/simde")

if(MLT_WITH_FASTPFOR)
    # FastPFor
    set(WITH_TEST OFF CACHE BOOL "Disable tests in FastPFor" FORCE) # The fastpfor gtest targets conflict with ours
    set(SUPPORT_NEON ON CACHE BOOL "" FORCE)

    if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.25")
        add_subdirectory("${PROJECT_SOURCE_DIR}/../vendor/fastpfor" "${CMAKE_CURRENT_BINARY_DIR}/fastpfor" EXCLUDE_FROM_ALL SYSTEM)
    else()
        add_subdirectory("${PROJECT_SOURCE_DIR}/../vendor/fastpfor" "${CMAKE_CURRENT_BINARY_DIR}/fastpfor" EXCLUDE_FROM_ALL)
        target_compile_options(FastPFOR PUBLIC "-Wno-cast-align")
    endif()

    target_link_libraries(mlt-cpp FastPFOR)
    target_include_directories(mlt-cpp PUBLIC "${PROJECT_SOURCE_DIR}/../vendor/fastpfor/headers")
    target_compile_definitions(mlt-cpp PUBLIC MLT_ENABLE_FASTPFOR=1)
    list(APPEND MLT_EXPORT_TARGETS FastPFOR)
endif(MLT_WITH_FASTPFOR)

# FSST
if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.25")
    add_subdirectory("${PROJECT_SOURCE_DIR}/../vendor/fsst" "${CMAKE_CURRENT_BINARY_DIR}/fsst" EXCLUDE_FROM_ALL SYSTEM)
else()
    add_subdirectory("${PROJECT_SOURCE_DIR}/../vendor/fsst" "${CMAKE_CURRENT_BINARY_DIR}/fsst" EXCLUDE_FROM_ALL)
endif()

target_include_directories(mlt-cpp PUBLIC "${PROJECT_SOURCE_DIR}/../vendor/fsst")

# Protozero
if(MLT_WITH_PROTOZERO)
    message(STATUS "[MLT] Including Protozero")
    set(DOXYGEN_FOUND OFF CACHE BOOL "Disable doxygen in protozero" FORCE)
    set(BUILD_TESTING OFF CACHE BOOL "Disable tests in protozero" FORCE)

    if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.25")
        add_subdirectory("${PROJECT_SOURCE_DIR}/../vendor/protozero" "${CMAKE_CURRENT_BINARY_DIR}/protozero" EXCLUDE_FROM_ALL SYSTEM)
    else()
        set(WERROR OFF CACHE BOOL "Disable warnings in protozero" FORCE)
        add_subdirectory("${PROJECT_SOURCE_DIR}/../vendor/protozero" "${CMAKE_CURRENT_BINARY_DIR}/protozero" EXCLUDE_FROM_ALL)
    endif()

    target_include_directories(mlt-cpp PUBLIC "${PROJECT_SOURCE_DIR}/../vendor/protozero/include")
    target_compile_definitions(mlt-cpp PUBLIC PROTOZERO_USE_VIEW=std::string_view MLT_WITH_PROTOZERO=1)
else()
    message(STATUS "[MLT] No Protozero")
endif(MLT_WITH_PROTOZERO)

# json
if(MLT_WITH_JSON)
    message(STATUS "[MLT] Including GeoJSON support")

    if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.25")
        add_subdirectory("${PROJECT_SOURCE_DIR}/../vendor/json" "${CMAKE_CURRENT_BINARY_DIR}/json" EXCLUDE_FROM_ALL SYSTEM)
    else()
        add_subdirectory("${PROJECT_SOURCE_DIR}/../vendor/json" "${CMAKE_CURRENT_BINARY_DIR}/json" EXCLUDE_FROM_ALL)
    endif()

    set(json_SOURCE_DIR "${PROJECT_SOURCE_DIR}/../vendor/json")
    target_include_directories(mlt-cpp PUBLIC "${json_SOURCE_DIR}/include")
    target_compile_definitions(mlt-cpp PUBLIC MLT_WITH_JSON=1)
else()
    message(STATUS "[MLT] No GeoJSON support")
endif(MLT_WITH_JSON)

if(MLT_WITH_PROTOZERO)
    add_subdirectory(${PROJECT_SOURCE_DIR}/test EXCLUDE_FROM_ALL)
    add_subdirectory(${PROJECT_SOURCE_DIR}/tool EXCLUDE_FROM_ALL)
endif()

export(TARGETS ${MLT_EXPORT_TARGETS} FILE MLTTargets.cmake)
